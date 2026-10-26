//! 恢复按钮只在当前已核验应用重建内部草稿；焦点变化后保留存档而非自动填入。

use super::QingjianInputController;
use super::output_guard::OutputGuard;
use crate::host;
use crate::imk::{TextClient, frontmost, secure_input};

impl QingjianInputController {
    pub(super) fn restore_saved_draft(
        &self,
        client: TextClient<'_>,
        expected: Option<OutputGuard<'_>>,
    ) {
        let state = host::with(|h| {
            (h.can_restore_from_menu() && expected.is_none_or(|guard| guard.valid(h)))
                .then(|| (h.live_observation_token(), h.live.raw_generation))
        })
        .flatten();
        let Some((observation, raw_generation)) = state else {
            return;
        };
        let application = client.bundle_identifier();
        let foreground = frontmost::matches(observation.application());
        if crate::imk::editor::session_owned()
            || !foreground
            || application.as_deref() != observation.application()
            || !self.is_current_controller()
            || secure_input::enabled()
        {
            return;
        }
        let restored = host::with(|h| {
            if !h.can_restore_from_menu()
                || !observation.matches(&h.live_observation_token())
                || expected.is_some_and(|guard| !guard.valid(h))
            {
                return None;
            }
            h.restore_draft()
        })
        .flatten();
        let Some((saved, restored_observation)) = restored else {
            return;
        };
        let foreground = frontmost::matches(Some(&saved.application));
        let still_current = !crate::imk::editor::session_owned()
            && foreground
            && self.is_current_controller()
            && !secure_input::enabled()
            && host::with(|h| restored_observation.matches(&h.live_observation_token()))
                .unwrap_or(false);
        if !still_current {
            host::with(|h| {
                h.retain_saved_draft(&saved, raw_generation);
                if restored_observation.matches(&h.live_observation_token()) {
                    h.cancel_draft("restore_context_changed");
                }
                h.live_action_notice("输入位置已变化，原草稿仍待明确恢复，未向应用填入文字");
            });
        }
    }
}

/// 原生菜单的客户端 IPC 在 Host 资格检查返回之后执行。
pub(crate) fn run_restore_if_ready(ready: impl FnOnce() -> bool, restore: impl FnOnce()) -> bool {
    if !ready() {
        return false;
    }
    restore();
    true
}

pub(crate) fn restore_current_draft_from_menu() {
    let _ = crate::imk::catch_panic("restoreMenuDraft", || {
        let Some(controller) = super::click::active_controller() else {
            return;
        };
        run_restore_if_ready(
            || host::with(|h| h.can_restore_from_menu()).unwrap_or(false),
            || {
                let client: Option<objc2::rc::Retained<objc2::runtime::AnyObject>> =
                    unsafe { objc2::msg_send![&*controller, client] };
                if let Some(client) = client
                    && controller.is_current_controller()
                    && !crate::imk::editor::session_owned()
                {
                    controller.restore_saved_draft(TextClient::new(&client), None);
                }
            },
        );
    });
}
