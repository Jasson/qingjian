//! 只从显式动作捕获当前目的地；普通重新显示不具备确认填入的权限。

use super::runtime::Runtime;
use super::target::Target;
use crate::host;
use crate::imk::{TextClient, controller, secure_input};
use objc2::MainThreadMarker;

pub(crate) fn open(client: TextClient<'_>, source: super::OpenSource) -> bool {
    if secure_input::enabled() {
        return false;
    }
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let Some(controller) = controller::editor_controller() else {
        return false;
    };
    let config = host::with(|h| {
        if !h.settings.config().live_translation.enabled {
            return None;
        }
        if !h.engine.composition().is_empty() && !h.draft_active() {
            return None;
        }
        Some(h.settings.config().clone())
    })
    .flatten();
    let Some(config) = config else {
        return false;
    };
    let Some((_, origin_pid)) = crate::imk::frontmost::identity() else {
        return false;
    };
    let previous = super::owner::Owner::current();
    super::owner::Owner::Opening(origin_pid).set("open_begin");
    let _rollback = super::opening_guard::OpeningGuard::new();
    let ticket = super::owner::Owner::generation();
    tracing::info!(target:"live_translation", marker="focused-owner-v1", "准备草稿窗口与可选目标");
    crate::imk::hotkey::disable();
    crate::imk::backspace_tap::disable();
    host::with(|h| {
        h.window.hide();
        h.live.window.hide();
    });
    let anchor = client.caret_rect();
    let target = Target::capture(controller, client);
    tracing::info!(target:"live_translation", marker="focused-owner-v1", captured=target.is_some(), "草稿目标捕获结束");
    if super::owner::Owner::generation() != ticket
        || super::owner::Owner::current() != super::owner::Owner::Opening(origin_pid)
    {
        tracing::info!(target:"live_translation", marker="focused-owner-v1", reason="application_changed", "草稿开窗已取消");
        return false;
    }
    let manual = super::STATE.with(|state| {
        state.try_borrow().ok().is_some_and(|state| {
            state.as_ref().is_some_and(|runtime| {
                runtime
                    .pending
                    .as_ref()
                    .is_some_and(|pending| pending.manual && pending.matches(&runtime.model))
                    && !runtime.writing
            })
        })
    });
    if manual && source == super::OpenSource::Shortcut {
        if let Some(target) = target {
            let confirmed = super::output::confirm_current(target);
            if !confirmed && super::owner::Owner::generation() == ticket {
                previous.set("confirmation_not_ready");
            }
            return true;
        }
        if super::owner::Owner::generation() == ticket {
            previous.set("confirmation_capture_failed");
        }
        return true;
    }
    let focused = present_prepared(mtm, target, origin_pid, anchor, &config);
    tracing::info!(target:"live_translation", marker="focused-owner-v1", focused, "草稿开窗结果");
    if focused {
        host::with(|h| {
            h.suspend_draft("open_focused_editor");
            h.window.hide();
            h.live.window.hide();
        });
        crate::imk::hotkey::disable();
        crate::imk::backspace_tap::disable();
    } else {
        if !super::session_owned() {
            host::with(|h| h.render());
        }
    }
    focused
}

/// 捕获目标之后的本地窗口步骤也供独立原生探针复用。
pub(super) fn present_prepared(
    mtm: MainThreadMarker,
    target: Option<Target>,
    origin_pid: i32,
    anchor: objc2_foundation::NSRect,
    config: &qingjian_platform::Config,
) -> bool {
    let prepared = super::STATE.with(|state| {
        let Ok(mut state) = state.try_borrow_mut() else {
            return None;
        };
        if let Some(runtime) = state.as_mut() {
            if runtime.writing {
                return None;
            }
            runtime.target = target;
            runtime.origin_pid = origin_pid;
            runtime.anchor = anchor;
            runtime.pending = None;
            runtime.status = "草稿保留在这里，编辑不会改变应用已有文字".into();
            runtime.render();
        } else {
            let mut runtime = Runtime::new(mtm, target, origin_pid, anchor, config);
            runtime.render();
            *state = Some(runtime);
        }
        let runtime = state.as_ref()?;
        super::CURRENT_PANEL.set(runtime.window.identity());
        Some((runtime.id, runtime.window.clone()))
    });
    let Some((id, window)) = prepared else {
        return false;
    };
    let ticket = super::owner::Owner::generation();
    let current = || {
        super::owner::Owner::generation() == ticket
            && super::owner::Owner::current() == super::owner::Owner::Opening(origin_pid)
            && !secure_input::enabled()
            && super::STATE.with(|state| {
                state.try_borrow().ok().is_some_and(|state| {
                    state.as_ref().is_some_and(|runtime| {
                        runtime.id == id && runtime.config.live_translation.enabled
                    })
                })
            })
    };
    let focused = window.focus(current) && current();
    if super::owner::Owner::generation() == ticket {
        super::KEYBOARD_OWNED.set(focused);
    }
    if focused {
        super::owner::Owner::Visible(origin_pid).set("window_presented");
    } else if super::owner::Owner::generation() == ticket {
        window.hide();
        super::STATE.with(|state| {
            if let Ok(mut state) = state.try_borrow_mut()
                && let Some(runtime) = state.as_mut()
                && runtime.id == id
            {
                runtime.status = "草稿窗未获得焦点，已保留原有输入，请重新打开".into();
                if super::owner::Owner::current() == super::owner::Owner::Opening(origin_pid) {
                    super::owner::Owner::Idle.set("open_failed");
                }
            }
        });
    }
    focused
}
