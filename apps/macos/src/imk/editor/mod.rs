//! 显式打开的独立草稿编辑器；输入焦点、输入会话和输出目的地均与普通 IMK 路线分离。

mod keyboard;
mod keys;
mod opening;
mod opening_guard;
mod output;
pub(crate) mod owner;
mod panel;
mod pending;
mod policy;
mod render;
mod runtime;
mod session;
mod smoke;
mod source;
mod target;
mod ticker;
mod view;
mod window;

use objc2_app_kit::NSEvent;
pub(crate) use opening::open;
use qingjian_render::{ActionTarget, HitTarget};
use runtime::Runtime;
pub(crate) use source::OpenSource;
use std::cell::{Cell, RefCell};

thread_local! {
    static STATE: RefCell<Option<Runtime>> = const { RefCell::new(None) };
    static CLOSE_PENDING: Cell<bool> = const { Cell::new(false) };
    static CURRENT_PANEL: Cell<usize> = const { Cell::new(0) };
    static KEYBOARD_OWNED: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn session_owned() -> bool {
    owner::Owner::controlled()
}

pub(crate) fn keyboard_owned() -> bool {
    KEYBOARD_OWNED.get()
}

pub(super) fn focus_lost(panel: usize) {
    if CURRENT_PANEL.get() != panel {
        return;
    }
    KEYBOARD_OWNED.set(false);
    tracing::info!(target: "live_translation", marker="focused-owner-v1", "草稿窗失去焦点");
    STATE.with(|state| {
        if let Ok(mut state) = state.try_borrow_mut()
            && let Some(runtime) = state.as_mut()
        {
            if !runtime.held_keys.is_empty()
                && let Some(pending) = runtime.pending.as_mut()
            {
                pending.manual = true;
            }
            runtime.window.hide();
            if let Some(pending) = runtime.pending.as_mut() {
                pending.manual = true;
            }
            if runtime.pending.is_some() {
                owner::Owner::Pending(runtime.origin_pid).set("focus_lost_pending");
            } else {
                owner::Owner::Idle.set("focus_lost_hidden");
            }
            runtime.status = "焦点已离开草稿窗，内容仍保留；未向应用填入文字".into();
        }
    });
}

pub(super) fn panel_focus_acquired(panel: usize) {
    if CURRENT_PANEL.get() == panel {
        focus_acquired();
    }
}

pub(super) fn focus_acquired() {
    if keyboard_owned() {
        return;
    }
    let (acquired, cancel_ready) = STATE.with(|state| {
        let Ok(mut state) = state.try_borrow_mut() else {
            return (false, false);
        };
        let Some(runtime) = state.as_mut() else {
            return (false, false);
        };
        runtime.prune_released_keys();
        (
            runtime.window.acquire_responder(),
            runtime.cancel_on_release && runtime.held_keys.is_empty(),
        )
    });
    if cancel_ready {
        discard();
        return;
    }
    KEYBOARD_OWNED.set(acquired);
    if acquired {
        crate::imk::hotkey::disable();
        crate::imk::backspace_tap::disable();
    }
}

pub(super) fn key_down(event: &NSEvent) {
    let _ = crate::imk::catch_panic("focusedDraftKey", || {
        if crate::imk::secure_input::enabled() {
            discard();
            return;
        }
        if CLOSE_PENDING.get() {
            discard();
            return;
        }
        sync_config();
        focus_acquired();
        let action = STATE.with(|state| state.try_borrow_mut().ok()?.as_mut()?.key(event));
        if let Some(action) = action {
            perform(action);
        }
    });
}

pub(super) fn key_up(event: &NSEvent) {
    let _ = crate::imk::catch_panic("focusedDraftKeyUp", || {
        let cancel = STATE.with(|state| {
            let Ok(mut state) = state.try_borrow_mut() else {
                return false;
            };
            let Some(runtime) = state.as_mut() else {
                return false;
            };
            runtime.held_keys.release(event.keyCode());
            if !runtime.held_keys.is_empty() {
                return false;
            }
            if runtime.cancel_on_release {
                return true;
            }
            if runtime
                .pending
                .as_ref()
                .is_some_and(|pending| !pending.manual)
            {
                KEYBOARD_OWNED.set(false);
                runtime.window.hide();
            }
            false
        });
        if cancel {
            discard();
        }
    });
}

pub(super) fn flags_changed(event: &NSEvent) {
    let _ = crate::imk::catch_panic("focusedDraftFlags", || {
        sync_config();
        focus_acquired();
        STATE.with(|state| {
            if let Ok(mut state) = state.try_borrow_mut()
                && let Some(runtime) = state.as_mut()
            {
                runtime.flags(event);
            }
        });
    });
}

pub(super) fn clicked(target: HitTarget) {
    let _ = crate::imk::catch_panic("focusedDraftClick", || {
        sync_config();
        focus_acquired();
        match target {
            HitTarget::Action(action) => perform(action),
            HitTarget::Candidate(index) => STATE.with(|state| {
                let Ok(mut state) = state.try_borrow_mut() else {
                    return;
                };
                let Some(runtime) = state.as_mut() else {
                    return;
                };
                if runtime.writing
                    || !runtime.window.owns_keyboard()
                    || index < runtime.candidate_start
                {
                    return;
                }
                let index = index - runtime.candidate_start + runtime.model.highlighted() / 9 * 9;
                runtime.pending = None;
                crate::host::with(|h| {
                    let mut engine =
                        session::SessionLease::new(&mut h.engine, &mut runtime.session);
                    runtime.model.choose(&mut engine, index);
                });
                runtime.submit();
                runtime.render();
            }),
            _ => {}
        }
    });
}

fn perform(action: ActionTarget) {
    match action {
        ActionTarget::InsertOriginal => output::request(true),
        ActionTarget::InsertTranslation => output::request(false),
        ActionTarget::CopyTranslation => {
            let text = STATE.with(|state| {
                state
                    .try_borrow()
                    .ok()?
                    .as_ref()?
                    .model
                    .copy_output()
                    .map(str::to_owned)
            });
            if let Some(text) = text {
                crate::host::copy_text_to_pasteboard(&text);
            }
        }
        ActionTarget::CancelDraft => cancel(),
        _ => {}
    }
}

fn discard() {
    CLOSE_PENDING.set(true);
    let runtime = STATE.with(|state| state.try_borrow_mut().ok().map(|mut state| state.take()));
    let Some(runtime) = runtime else {
        tracing::debug!(target:"live_translation", marker="focused-owner-v1", reason="state_busy", "草稿关闭延后，保持会话控制权");
        return;
    };
    CLOSE_PENDING.set(false);
    CURRENT_PANEL.set(0);
    KEYBOARD_OWNED.set(false);
    drop(runtime);
    owner::Owner::Idle.set("discard");
}

fn cancel() {
    let wait = STATE.with(|state| {
        let Ok(mut state) = state.try_borrow_mut() else {
            return true;
        };
        let Some(runtime) = state.as_mut() else {
            return false;
        };
        if runtime.held_keys.is_empty() {
            return false;
        }
        runtime.pending = None;
        runtime.cancel_on_release = true;
        runtime.status = "草稿已取消，请松开按键后关闭窗口".into();
        runtime.render();
        true
    });
    if !wait {
        discard();
    }
}

pub(super) fn tick() {
    let _ = crate::imk::catch_panic("focusedDraftPoll", || {
        if CLOSE_PENDING.get() {
            discard();
            return;
        }
        if crate::imk::secure_input::enabled() {
            discard();
            return;
        }
        sync_frontmost();
        sync_config();
        STATE.with(|state| {
            let Ok(mut state) = state.try_borrow_mut() else {
                return;
            };
            let Some(runtime) = state.as_mut() else {
                return;
            };
            if runtime.reconfigure_input {
                let refreshed = crate::host::with(|h| {
                    let mut engine =
                        session::SessionLease::new(&mut h.engine, &mut runtime.session);
                    runtime.model.commit_raw(&mut engine);
                    runtime.model.refresh(&mut engine);
                })
                .is_some();
                if refreshed {
                    runtime.reconfigure_input = false;
                    runtime.submit();
                    runtime.render();
                }
            }
            runtime.submit();
            if let Some(result) = runtime
                .translator
                .as_mut()
                .and_then(|translator| translator.poll())
                && runtime.model.accept(result)
            {
                runtime.render();
            }
        });
        output::attempt();
    });
}

fn sync_config() {
    if let Some(config) = crate::host::with(|h| h.settings.config().clone()) {
        configuration_changed(&config);
    }
}

pub(crate) fn configuration_changed(config: &qingjian_platform::Config) {
    if !config.live_translation.enabled {
        discard();
        return;
    }
    STATE.with(|state| {
        let Ok(mut state) = state.try_borrow_mut() else {
            return;
        };
        let Some(runtime) = state.as_mut() else {
            return;
        };
        if runtime.config == *config {
            return;
        }
        runtime.pending = None;
        runtime.writing = false;
        if matches!(owner::Owner::current(), owner::Owner::Pending(_)) {
            runtime.window.hide();
            owner::Owner::Idle.set("settings_cancel_confirmation");
        }
        let raw_changed = runtime.config.general.scheme() != config.general.scheme()
            || runtime.config.general.wubi() != config.general.wubi()
            || runtime.config.general.shift_letter != config.general.shift_letter
            || runtime.config.general.english_mode != config.general.english_mode;
        runtime.config = config.clone();
        runtime.shortcut = config.shortcut;
        runtime
            .mode
            .configure(config.general.english_mode, config.shortcut.switch_mode);
        runtime.model.invalidate_translation();
        runtime.translator =
            qingjian_predict::live::LiveTranslator::new(config.live_translation.clone()).ok();
        runtime.submitted_revision = None;
        runtime.status = "设置已更新；待确认填入已撤销，待选原键将按字面保留".into();
        runtime.reconfigure_input |= raw_changed;
    });
}

#[cfg(test)]
mod tests;

pub(crate) use smoke::run as smoke;

/// 前台切到另一个应用时先隐藏保留草稿，再释放旧输入路径的控制权。
pub(crate) fn sync_frontmost() {
    let Some((_, pid)) = crate::imk::frontmost::identity() else {
        return;
    };
    if !owner::Owner::current().departed(pid) {
        return;
    }
    let hidden = STATE.with(|state| {
        let Ok(mut state) = state.try_borrow_mut() else {
            return false;
        };
        if let Some(runtime) = state.as_mut() {
            runtime.pending = None;
            runtime.writing = false;
            runtime.window.hide();
        }
        true
    });
    if hidden {
        KEYBOARD_OWNED.set(false);
        owner::Owner::Idle.set("application_changed");
    }
}

/// 编辑器会话先于普通 IMK 客户端准备，避免旧草稿兜底重新接管。
pub(crate) fn intercept_host(event: &NSEvent, client: crate::imk::TextClient<'_>) -> Option<bool> {
    sync_frontmost();
    if !session_owned() {
        return None;
    }
    let phase = owner::Owner::current();
    if event.r#type() != objc2_app_kit::NSEventType::KeyDown {
        return Some(false);
    }
    let shortcut = STATE.with(|state| {
        state
            .try_borrow()
            .ok()
            .map(|state| state.as_ref().map(|runtime| runtime.shortcut))
    });
    let Some(shortcut) = shortcut else {
        return Some(true);
    };
    if matches!(phase, owner::Owner::Pending(_))
        && let Some(shortcut) = shortcut
        && keyboard::matches(shortcut.open_translation_draft, event)
    {
        if !event.isARepeat() {
            open(client, OpenSource::Shortcut);
        }
        return Some(true);
    }
    if matches!(phase, owner::Owner::Pending(_)) {
        if let Some(shortcut) = shortcut {
            if keyboard::action_matches(shortcut.cancel_draft, event) {
                discard();
                return Some(true);
            }
            if keyboard::matches(shortcut.insert_live_translation, event)
                || keyboard::action_matches(shortcut.insert_original, event)
            {
                STATE.with(|state| {
                    if let Ok(mut state) = state.try_borrow_mut()
                        && let Some(runtime) = state.as_mut()
                    {
                        runtime.status =
                            "请回到要填入的输入框，再按开窗快捷键明确确认当前位置".into();
                        runtime.render();
                        runtime.window.show_notice();
                    }
                });
                return Some(true);
            }
        }
        STATE.with(|state| {
            if let Ok(mut state) = state.try_borrow_mut()
                && let Some(runtime) = state.as_mut()
            {
                runtime.pending = None;
                runtime.window.hide();
            }
        });
        owner::Owner::Idle.set("host_edit_cancels_confirmation");
        return None;
    }
    Some(true)
}
