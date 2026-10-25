//! 仅在当前符合应用策略的草稿会话注册系统热键；Carbon 回调不直接读写应用。

mod binding;
mod event_type;
mod id;
mod native;
mod operation_guard;
mod state;
mod ticker;

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_foundation::NSTimer;
use qingjian_platform::KeyCombo;

use crate::host::{self, ObservationToken};
use crate::imk::{controller, secure_input};
use binding::Binding;
use operation_guard::OperationGuard;
use state::State;

thread_local! {
    static BUSY: Cell<bool> = const { Cell::new(false) };
    static STATE: RefCell<State<ObservationToken>> = RefCell::new(State::default());
    static TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
    static HANDLER: RefCell<Option<usize>> = const { RefCell::new(None) };
}

pub(crate) fn tracking() -> bool {
    STATE.with(|state| state.borrow().tracking())
}

pub(crate) fn publish_observation(token: ObservationToken) {
    let dropped = STATE.with(|state| state.borrow_mut().publish(token));
    if dropped {
        tracing::info!(target: "live_translation", reason = "input_changed", "系统译文热键排队意图已失效");
    }
}

pub(crate) fn owns(combo: KeyCombo) -> bool {
    STATE.with(|state| state.borrow().owns(combo))
}

pub(crate) fn disable() {
    let _ = crate::imk::catch_panic("注销系统译文热键", disable_inner);
}

fn disable_inner() {
    if MainThreadMarker::new().is_none() {
        return;
    }
    reconcile(None, None);
    update_timer();
}

pub(crate) fn activate() {
    sync();
}

fn update_timer() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let needed = STATE.with(|state| state.borrow().needs_monitor());
    let exists = TIMER.with(|timer| timer.borrow().is_some());
    if needed && !exists {
        let target = ticker::Ticker::new(mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                0.05,
                &target,
                sel!(tick:),
                None,
                true,
            )
        };
        TIMER.with(|slot| *slot.borrow_mut() = Some(timer));
    } else if !needed && exists {
        let timer = TIMER.with(|slot| slot.borrow_mut().take());
        if let Some(timer) = timer {
            timer.invalidate();
        }
    }
}

pub(crate) fn sync() {
    let _ = crate::imk::catch_panic("同步系统译文热键", sync_inner);
}

fn sync_inner() {
    if crate::imk::editor::session_owned() {
        disable();
        return;
    }
    if MainThreadMarker::new().is_none() {
        return;
    }
    let Some(owner) = controller::active_controller_id() else {
        disable();
        return;
    };
    if secure_input::enabled() {
        disable();
        return;
    }
    let foreground = crate::imk::frontmost::identity().filter(|(_, pid)| *pid > 0);
    // 借不到 Host 时保留意图，下一轮再核验，不能把“正在重入”误当关闭。
    if controller::active_controller_id() != Some(owner) {
        disable();
        return;
    }
    let desired = host::with(|h| {
        let config = h.settings.config();
        let binding = if h.draft_active()
            && crate::imk::draft_policy::matches(
                &config.live_translation,
                h.engine.application(),
                foreground.as_ref().map(|(app, _)| app.as_str()),
                false,
            ) {
            Binding::for_session(
                config.shortcut.insert_live_translation,
                Some(owner),
                h.engine.application(),
                &config.live_translation,
            )
        } else {
            None
        };
        (binding, h.live_observation_token())
    });
    if controller::active_controller_id() != Some(owner) {
        disable();
        return;
    }
    if let Some((binding, observation)) = desired {
        reconcile(binding, Some(observation));
        update_timer();
    }
}

fn reconcile(binding: Option<Binding>, observation: Option<ObservationToken>) {
    STATE.with(|state| state.borrow_mut().change(binding));
    if let Some(observation) = observation {
        publish_observation(observation);
    }
    let Some(_operation) = OperationGuard::enter() else {
        return;
    };
    if !retire() {
        publish_notice();
        return;
    }
    let request = STATE.with(|state| state.borrow_mut().registration_request());
    let Some((generation, binding)) = request else {
        publish_notice();
        return;
    };
    let installed = HANDLER.with(|handler| *handler.borrow());
    let installed = match installed {
        Some(handle) => Ok(handle),
        None => {
            let result = native::install();
            if let Ok(handle) = result {
                HANDLER.with(|handler| *handler.borrow_mut() = Some(handle));
            }
            result
        }
    };
    let result = installed.and_then(|_| native::register(binding, generation));
    let status = result.as_ref().err().copied();
    STATE.with(|state| state.borrow_mut().complete(generation, result));
    if let Some(status) = status {
        tracing::warn!(target: "live_translation", status, generation,
            key = %binding.combo.key, modifiers = binding.modifiers(), "系统译文热键注册失败，仍可使用按钮");
    } else if current(generation) {
        tracing::info!(target: "live_translation", generation,
            key = %binding.combo.key, modifiers = binding.modifiers(), "系统译文热键已排他注册");
    }
    retire();
    publish_notice();
}

fn retire() -> bool {
    let handle = STATE.with(|state| state.borrow().retiring());
    let Some(handle) = handle else {
        return true;
    };
    match native::unregister(handle) {
        Ok(()) => {
            STATE.with(|state| state.borrow_mut().retired(handle));
            tracing::info!(target: "live_translation", "系统译文热键已注销");
            true
        }
        Err(status) => {
            if STATE.with(|state| state.borrow_mut().retirement_failed(status)) {
                tracing::error!(target: "live_translation", status, "系统译文热键注销失败，将继续重试");
            }
            false
        }
    }
}

fn publish_notice() {
    let notice = STATE
        .with(|state| state.borrow().error())
        .map(|(operation, status)| {
            format!("系统热键{operation}失败（OSStatus {status}），请使用输出译文按钮")
        });
    host::with(|h| {
        if h.live.system_hotkey_notice != notice {
            h.live.system_hotkey_notice = notice;
            h.refresh_live_actions();
        }
    });
}

fn receive(generation: u32, pressed: bool) {
    let accepted = STATE.with(|state| state.borrow_mut().receive(generation, pressed));
    if pressed && accepted {
        tracing::info!(target: "live_translation", generation, source = "carbon", reason = "queued", "收到系统译文热键");
    } else if pressed {
        tracing::debug!(target: "live_translation", generation, reason = "stale_or_repeat", "忽略系统译文热键");
    }
}

fn poll() {
    sync();
    let pending = STATE.with(|state| state.borrow_mut().start_dispatch());
    let Some((generation, observation)) = pending else {
        return;
    };
    let binding = STATE.with(|state| state.borrow().binding(generation));
    let Some(binding) = binding else {
        return;
    };
    let retry = !controller::dispatch_system_translation_hotkey(
        generation,
        binding.owner,
        binding.combo,
        &observation,
    );
    STATE.with(|state| state.borrow_mut().finish(generation, retry));
}

pub(crate) fn current(generation: u32) -> bool {
    STATE.with(|state| state.borrow().current(generation))
}

pub(crate) fn binding_combo(generation: u32) -> Option<KeyCombo> {
    STATE.with(|state| {
        state
            .borrow()
            .binding(generation)
            .map(|binding| binding.combo)
    })
}
