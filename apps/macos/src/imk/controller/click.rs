//! 鼠标点候选窗口：点候选与按它的数字键一样上屏，点译词与按译词键一样上屏它的那条译词。
//!
//! 候选窗口不知道是哪个会话，所以在 `activateServer:` 时记下当前控制器，点击时用它的客户端上屏。

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{Message, msg_send};
use qingjian_render::HitTarget;

use crate::candidates::PanelMouse;

use super::QingjianInputController;
use crate::host;
use crate::imk::{TextClient, catch_panic, recover_from_panic};

thread_local! {
    /// 最近激活的会话；`deactivateServer:` 时清掉。
    static ACTIVE: RefCell<Option<Retained<QingjianInputController>>> = const { RefCell::new(None) };
}

pub(crate) fn active_controller_id() -> Option<usize> {
    ACTIVE.with(|active| {
        active
            .borrow()
            .as_ref()
            .map(|controller| &**controller as *const QingjianInputController as usize)
    })
}

pub(super) fn active_controller() -> Option<Retained<QingjianInputController>> {
    ACTIVE.with(|active| active.borrow().clone())
}

/// 只有 Host 暂时借用失败才返回 false 以延迟；门禁拒绝不应自动等模型后输出。
pub(crate) fn dispatch_system_translation_hotkey(
    generation: u32,
    owner: usize,
    combo: qingjian_platform::KeyCombo,
    observation: &host::ObservationToken,
) -> bool {
    use crate::imk::{hotkey, secure_input};
    if crate::imk::editor::session_owned() {
        return true;
    }
    let Some(controller) = ACTIVE.with(|active| active.borrow().clone()) else {
        return true;
    };
    if active_controller_id() != Some(owner) || !hotkey::current(generation) {
        tracing::info!(target: "live_translation", generation, reason = "controller_or_registration_changed", "系统译文热键未输出");
        return true;
    }
    if secure_input::enabled() || !crate::imk::frontmost::matches(observation.application()) {
        hotkey::disable();
        tracing::info!(target: "live_translation", generation, reason = "secure_or_not_frontmost", "系统译文热键未输出");
        return true;
    }
    let valid = host::with(|h| {
        h.draft_active()
            && h.settings.config().shortcut.insert_live_translation == Some(combo)
            && observation.matches(&h.live_observation_token())
    });
    let Some(valid) = valid else {
        return false;
    };
    if !valid {
        tracing::info!(target: "live_translation", generation, reason = "input_or_configuration_changed", "系统译文热键未输出");
        return true;
    }
    let client: Option<Retained<AnyObject>> = unsafe { msg_send![&*controller, client] };
    let Some(client) = client else {
        return true;
    };
    if crate::imk::editor::session_owned()
        || !controller.is_current_controller()
        || !hotkey::current(generation)
    {
        return true;
    }
    if secure_input::enabled() || !crate::imk::frontmost::matches(observation.application()) {
        hotkey::disable();
        tracing::info!(target: "live_translation", generation, reason = "secure_or_not_frontmost", "系统译文热键未输出");
        return true;
    }
    let valid = host::with(|h| {
        h.draft_active()
            && h.settings.config().shortcut.insert_live_translation == Some(combo)
            && observation.matches(&h.live_observation_token())
    });
    let Some(valid) = valid else {
        return false;
    };
    if !valid {
        tracing::info!(target: "live_translation", generation, reason = "input_or_configuration_changed", "系统译文热键未输出");
        return true;
    }
    controller.perform_registered_live_action(TextClient::new(&client), generation, observation);
    true
}

/// 候选窗口的点击回调（host 建窗口时注册）。
pub fn click(target: HitTarget) {
    if crate::imk::editor::session_owned() {
        return;
    }
    let Some(controller) = ACTIVE.with(|active| active.borrow().clone()) else {
        tracing::debug!("没有激活的会话，点击不处理");
        return;
    };
    if catch_panic("click", || controller.click(target)).is_none() {
        recover_from_panic(None);
    }
}

/// 符号面板的点击回调（host 建窗口时注册）。
pub fn panel_mouse(hit: PanelMouse) {
    if crate::imk::editor::session_owned() {
        return;
    }
    let Some(controller) = ACTIVE.with(|active| active.borrow().clone()) else {
        return;
    };
    // SAFETY: IMKInputController 的 `client` 返回当前会话的文本客户端（实现 IMKTextInput 的代理），可能为 nil
    let client: Option<Retained<AnyObject>> = unsafe { msg_send![&*controller, client] };
    let Some(client) = client else {
        return;
    };
    if crate::imk::editor::session_owned() {
        return;
    }
    if catch_panic("panel_click", || {
        controller.panel_mouse_event(hit, TextClient::new(&client))
    })
    .is_none()
    {
        recover_from_panic(None);
    }
}

/// 译文就绪后刷新一次动作能力，覆盖英文直输与标点随后才由应用插入的路径。
pub fn refresh_live_actions() {
    if crate::imk::editor::session_owned() {
        return;
    }
    let Some(controller) = ACTIVE.with(|active| active.borrow().clone()) else {
        return;
    };
    let epoch = host::with(|h| h.live.epoch);
    let client: Option<Retained<AnyObject>> = unsafe { msg_send![&*controller, client] };
    let Some(client) = client else {
        return;
    };
    if crate::imk::editor::session_owned()
        || !controller.is_current_controller()
        || host::with(|h| h.live.epoch) != epoch
    {
        return;
    }
    if catch_panic("live_actions", || {
        if controller.observe_live_context(TextClient::new(&client))
            && controller.is_current_controller()
        {
            host::with(|h| h.refresh_live_actions());
        }
    })
    .is_none()
    {
        recover_from_panic(None);
    }
}

impl QingjianInputController {
    pub(super) fn set_active(&self) {
        ACTIVE.with(|active| *active.borrow_mut() = Some(self.retain()));
    }

    pub(super) fn clear_active(&self) {
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            if active
                .as_deref()
                .is_some_and(|current| std::ptr::eq(current, self))
            {
                *active = None;
            }
        });
    }

    pub(super) fn is_current_controller(&self) -> bool {
        ACTIVE.with(|active| {
            active
                .borrow()
                .as_deref()
                .is_some_and(|current| std::ptr::eq(current, self))
        })
    }

    fn click(&self, target: HitTarget) {
        if crate::imk::editor::session_owned() {
            return;
        }
        let observation = matches!(target, HitTarget::Action(_))
            .then(|| host::with(|h| h.live_observation_token()))
            .flatten();
        // SAFETY: IMKInputController 的 `client` 返回当前会话的文本客户端（实现 IMKTextInput 的代理），可能为 nil
        let client: Option<Retained<AnyObject>> = unsafe { msg_send![self, client] };
        let Some(client) = client else {
            return;
        };
        if crate::imk::editor::session_owned() {
            return;
        }
        let client = TextClient::new(&client);
        if let HitTarget::Action(action) = target {
            let Some(observation) = observation else {
                return;
            };
            if host::with(|h| observation.matches(&h.live_observation_token())) != Some(true)
                || !self.is_current_controller()
            {
                return;
            }
            self.perform_live_action_checked(
                action,
                Some(client),
                Some(super::output_guard::OutputGuard::Snapshot(&observation)),
            );
            return;
        }
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            return;
        }
        match target {
            HitTarget::Candidate(offset) => {
                if let Some(index) = host::with(|h| h.session.index_in_view(offset)).flatten() {
                    self.commit_index(index, client);
                }
            }
            HitTarget::Action(_) => {}
            HitTarget::Translation { candidate, sense } => {
                let index = host::with(|h| h.session.index_in_view(candidate)).flatten();
                self.commit_translation_at(index, sense, client);
            }
        }
    }
}
