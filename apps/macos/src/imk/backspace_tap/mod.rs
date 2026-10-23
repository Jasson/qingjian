//! Terminal 内部草稿的普通退格接管；active ForPid tap 仅排队，不写应用文本。

mod action;
mod batch;
mod bindings;
mod entry;
mod event;
mod held;
mod key_binding;
mod native;
mod operation_guard;
mod state;
mod ticker;

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::NSWorkspace;
use objc2_foundation::NSTimer;

use crate::host::{self, ObservationToken};
use crate::imk::{controller, secure_input};
pub(crate) use action::Action;
pub(crate) use batch::Batch;
pub(crate) use bindings::Bindings;
use event::Event;
use operation_guard::OperationGuard;
use state::State;

thread_local! {
    static BUSY: Cell<bool> = const { Cell::new(false) };
    static STATE: RefCell<State<ObservationToken>> = RefCell::new(State::default());
    static TAP: RefCell<Option<native::Tap>> = const { RefCell::new(None) };
    static TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
}

pub(crate) fn tracking() -> bool {
    TIMER.with(|timer| timer.borrow().is_some())
}

/// Host 释放借用后同步发布；首次启用才装轮询，不重新借用 Host。
pub(crate) fn publish(
    token: ObservationToken,
    configured: bool,
    content: bool,
    bindings: Bindings,
) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.publish_bindings(bindings);
        state.publish(token, configured, content);
    });
    if !configured {
        disable();
        return;
    }
    if !tracking() {
        if !STATE.with(|state| state.borrow().wants_monitor()) {
            return;
        }
        // NSWorkspace 与 NSTimer 都可能跑嵌套 run-loop；重入只发布新镜像。
        let Some(_guard) = OperationGuard::enter() else {
            return;
        };
        let generation = STATE.with(|state| state.borrow().generation);
        let owner = controller::active_controller_id();
        let pid = if secure_input::enabled() {
            None
        } else {
            frontmost_pid()
        };
        if owner.is_none() || pid.is_none() || controller::active_controller_id() != owner {
            return;
        }
        let current = STATE.with(|state| {
            let mut state = state.borrow_mut();
            if state.generation != generation || !state.configured {
                return None;
            }
            let token = state.token.clone()?;
            let content = state.content;
            state.bind(owner, pid);
            state.publish(token, true, content);
            Some(state.generation)
        });
        if let Some(generation) = current {
            start_timer(generation);
        }
    }
}

pub(crate) fn activate() {
    let _ = crate::imk::catch_panic("激活草稿退格接管", activate_inner);
}

fn activate_inner() {
    disable();
    STATE.with(|state| state.borrow_mut().failed = None);
    host::with(|h| {
        if h.live.backspace_tap_notice.take().is_some() {
            h.refresh_live_actions();
        }
    });
    if tracking() {
        sync();
    }
}

fn start_timer(generation: u64) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let target = ticker::Ticker::new(mtm);
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            0.01,
            &target,
            sel!(tick:),
            None,
            true,
        )
    };
    let current = STATE.with(|state| {
        let state = state.borrow();
        state.generation == generation && state.configured
    });
    if current && !tracking() {
        TIMER.with(|slot| *slot.borrow_mut() = Some(timer));
    } else {
        timer.invalidate();
    }
}

pub(crate) fn disable() {
    let _ = crate::imk::catch_panic("停止草稿退格接管", disable_inner);
}

fn disable_inner() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.revoke();
        state.owner = None;
        state.pid = None;
    });
    let timer = TIMER.with(|slot| slot.borrow_mut().take());
    if let Some(timer) = timer {
        timer.invalidate();
    }
    retire();
}

fn retire() {
    let Some(_guard) = OperationGuard::enter() else {
        return;
    };
    let tap = TAP.with(|slot| slot.borrow_mut().take());
    drop(tap);
}

fn stop_idle_timer() {
    let idle = !STATE.with(|state| state.borrow().wants_monitor())
        && TAP.with(|tap| tap.borrow().is_none());
    if idle {
        let timer = TIMER.with(|slot| slot.borrow_mut().take());
        if let Some(timer) = timer {
            timer.invalidate();
        }
    }
}

pub(crate) fn frontmost_pid() -> Option<i32> {
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    (app.bundleIdentifier()?.to_string() == "com.apple.Terminal").then(|| app.processIdentifier())
}

fn sync() {
    let owner = controller::active_controller_id();
    let pid = if secure_input::enabled() {
        None
    } else {
        frontmost_pid()
    };
    STATE.with(|state| state.borrow_mut().bind(owner, pid));
    if owner.is_none() || pid.is_none() {
        disable();
        return;
    }
    // Host 每次释放借用都会发布镜像；轮询负责创建或释放 native 资源。
    host::with(|_| ());
    let desired = STATE.with(|state| {
        let state = state.borrow();
        state.needs_tap().then_some(state.generation)
    });
    let existing = TAP.with(|tap| tap.borrow().as_ref().map(|tap| tap.generation));
    if existing == desired {
        publish_notice();
        stop_idle_timer();
        return;
    }
    let Some(_guard) = OperationGuard::enter() else {
        return;
    };
    let old = TAP.with(|tap| tap.borrow_mut().take());
    STATE.with(|state| state.borrow_mut().installed = false);
    drop(old);
    if let Some(generation) = desired {
        let tap = native::create(pid.unwrap(), generation);
        let current = STATE.with(|state| {
            let state = state.borrow();
            state.generation == generation && state.needs_tap()
        });
        if current {
            if let Some(tap) = tap {
                TAP.with(|slot| *slot.borrow_mut() = Some(tap));
                STATE.with(|state| state.borrow_mut().installed = true);
                tracing::info!(target: "live_translation", generation, "普通草稿退格接管已启用");
            } else {
                STATE.with(|state| {
                    state.borrow_mut().fail(
                        "草稿按键接管不可用，请检查青简的辅助功能与输入监控权限，再重新激活青简",
                    )
                });
            }
        }
    }
    publish_notice();
    stop_idle_timer();
}

fn publish_notice() {
    let notice = STATE.with(|state| state.borrow().failed.map(str::to_owned));
    host::with(|h| {
        if h.live.backspace_tap_notice != notice {
            h.live.backspace_tap_notice = notice;
            h.refresh_live_actions();
        }
    });
}

fn receive(event: Event) -> bool {
    STATE.with(|state| {
        state
            .try_borrow_mut()
            .is_ok_and(|mut state| state.receive(event))
    })
}

pub(crate) fn owns_imk(timestamp_seconds: f64, key: u16) -> bool {
    if !timestamp_seconds.is_finite() || timestamp_seconds <= 0.0 {
        return false;
    }
    let timestamp = (timestamp_seconds * 1_000_000_000.0) as u64;
    STATE.with(|state| state.borrow().owns_imk_key(timestamp, i64::from(key)))
}

pub(crate) fn current(generation: u64, serial: u64) -> bool {
    STATE.with(|state| state.borrow().current(generation, serial))
}

pub(crate) fn action_consumed(generation: u64, serial: u64) {
    STATE.with(|state| state.borrow_mut().action_consumed(generation, serial));
}

pub(crate) fn release_imk(key: u16) -> bool {
    STATE.with(|state| state.borrow_mut().release_imk(i64::from(key)))
}

pub(crate) fn queue_imk_action(key: i64, flags: u64, repeat: bool, timestamp: f64) -> Option<bool> {
    let wanted = STATE.with(|state| state.borrow().wants_imk_action(key, flags, repeat));
    if !wanted {
        return None;
    }
    let ticket = STATE.with(|state| state.borrow().imk_ticket())?;
    // 首输入后可能尚未轮到 timer；先建立可隔离重复按键的 native 入口再排动作。
    sync();
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if !state.input_matches(ticket.0, ticket.1, &ticket.2) {
            return;
        }
        let event = Event {
            generation: state.generation,
            kind: 10,
            key,
            flags,
            pid: state.pid.map_or(0, i64::from),
            timestamp: (timestamp * 1_000_000_000.0) as u64,
            repeat,
            secure: secure_input::enabled(),
        };
        state.receive(event);
    });
    // 创建失败时保留草稿并显示权限提示，不能输出后放任重复 Enter 执行命令。
    Some(true)
}

fn poll() {
    sync();
    let batch = STATE.with(|state| state.borrow_mut().batch());
    let Some(batch) = batch else { return };
    if controller::active_controller_id() != Some(batch.owner)
        || secure_input::enabled()
        || frontmost_pid() != Some(batch.pid)
    {
        STATE.with(|state| state.borrow_mut().revoke());
        return;
    }
    let current = STATE.with(|state| state.borrow().current(batch.generation, batch.serial));
    if !current {
        return;
    }
    STATE.with(|state| state.borrow_mut().dispatching = true);
    let applied = controller::dispatch_draft_tap(&batch);
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.dispatching = false;
        state.finish(
            batch.generation,
            batch.serial,
            batch.count,
            applied == Some(true),
        );
        if applied == Some(false) {
            state.discard_pending();
        }
    });
    if applied == Some(false) {
        tracing::info!(target: "live_translation", reason = "session_or_input_changed", "已丢弃失效的草稿退格批次");
    } else if applied.is_none() {
        tracing::debug!(target: "live_translation", "Host 暂忙，保留草稿退格批次等待下轮");
    } else {
        tracing::debug!(target: "live_translation", count = batch.count, "草稿退格批次已处理");
    }
    sync();
}
