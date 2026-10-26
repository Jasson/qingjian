//! 微信草稿的有界元数据诊断；不读取文本、不改变路由，失败仅放弃诊断记录。

mod callback;
mod counts;
mod event;
mod frame;
mod kind;
mod record;
pub(crate) mod snapshot;
mod state;

pub(crate) use kind::WriteKind;

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use objc2_app_kit::{NSEvent, NSEventType};

use super::QingjianInputController;
use super::secure_input;
use crate::host;
use event::Event;
use record::Record;
use state::State;

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn isolated<T>(action: impl FnOnce() -> T) -> Option<T> {
    catch_unwind(AssertUnwindSafe(action)).ok()
}

pub(crate) fn arm(controller: &QingjianInputController) {
    let _ = isolated(|| {
        let eligible = !secure_input::enabled()
            && host::draft_diagnostic_snapshot().is_some_and(|value| value.wechat);
        STATE.with(|state| {
            if let Ok(mut state) = state.try_borrow_mut() {
                state.arm(
                    controller as *const QingjianInputController as usize,
                    eligible,
                );
            }
        });
    });
}

pub(crate) fn disarm(controller: &QingjianInputController) {
    let _ = isolated(|| {
        STATE.with(|state| {
            if let Ok(mut state) = state.try_borrow_mut() {
                state.disarm(controller as *const QingjianInputController as usize);
            }
        })
    });
}

pub(crate) fn begin(
    event: Option<&NSEvent>,
    client_present: bool,
    current: impl FnOnce() -> bool,
) -> Event {
    let created = isolated(|| {
        let secure = secure_input::enabled();
        let before = (!secure).then(host::draft_diagnostic_snapshot).flatten();
        let current = current();
        let kind = match event.map(NSEvent::r#type) {
            Some(NSEventType::KeyDown) => "down",
            Some(NSEventType::KeyUp) => "up",
            Some(NSEventType::FlagsChanged) => "flags",
            Some(
                NSEventType::LeftMouseDown
                | NSEventType::RightMouseDown
                | NSEventType::OtherMouseDown,
            ) => "mouse",
            Some(_) => "other",
            None => "missing",
        };
        let (ticket, record) = STATE.with(|state| {
            let mut state = state.try_borrow_mut().ok()?;
            let ticket = state.begin(kind, before, current, client_present, secure);
            Some((ticket, state.record_mut(ticket).cloned()))
        })?;
        Some((ticket, record))
    })
    .flatten();
    let ticket = created.as_ref().map(|(ticket, _)| *ticket);
    if let Some((_, Some(record))) = created {
        let _ = isolated(|| {
            if secure_input::enabled() {
                return;
            }
            let before = record.before;
            tracing::info!(target: "live_translation", phase = "begin", event_id = record.id,
                session = record.session, kind = record.kind, current_controller = record.current_before,
                client_present = record.client_present,
                host_available = before.is_some(), staged = before.map(|value| value.staged),
                epoch = before.map(|value| value.epoch), composing = before.map(|value| value.composing),
                draft_present = before.map(|value| value.draft_present),
                pid = std::process::id(), version = env!("CARGO_PKG_VERSION"),
                build = option_env!("QINGJIAN_BUILD").unwrap_or("本地构建"),
                diagnostic_revision = "wechat-draft-v2", "微信草稿诊断");
        });
    }
    Event { ticket }
}

fn update(ticket: Option<u64>, action: impl FnOnce(&mut Record)) {
    let _ = isolated(|| {
        STATE.with(|state| {
            if let Some(ticket) = ticket
                && let Ok(mut state) = state.try_borrow_mut()
                && let Some(record) = state.record_mut(ticket)
            {
                action(record);
            }
        })
    });
}

pub(crate) fn write(kind: WriteKind, outcome: &'static str) {
    let _ = isolated(|| {
        if secure_input::enabled() {
            return;
        }
        STATE.with(|state| {
            if let Ok(mut state) = state.try_borrow_mut() {
                state.write(kind, outcome);
            }
        });
    });
}

fn finish(ticket: Option<u64>, handled: Option<bool>, current: impl FnOnce() -> bool) {
    let _ = isolated(|| {
        let record = STATE.with(|state| state.try_borrow_mut().ok()?.finish(ticket?));
        let Some(record) = record else {
            return;
        };
        if secure_input::enabled() {
            return;
        }
        let after = host::draft_diagnostic_snapshot();
        // 身份已明确切出微信时不再写这条记录；借用忙只保留有限元数据。
        if after.is_some_and(|value| !value.wechat) {
            return;
        }
        let [marked, ordinary, confirmed, empty_marked] = record.writes;
        tracing::info!(target: "live_translation", phase = "end", event_id = record.id,
            session = record.session, kind = record.kind, prepared = record.prepared,
            handled, completed = handled.is_some(), panicked = record.panicked || handled.is_none(),
            current_before = record.current_before, current_after = current(),
            client_present = record.client_present, host_available = after.is_some(),
            staged_before = record.before.map(|value| value.staged), staged_after = after.map(|value| value.staged),
            epoch_before = record.before.map(|value| value.epoch), epoch_after = after.map(|value| value.epoch),
            composing = after.map(|value| value.composing), draft_present = after.map(|value| value.draft_present),
            marked_attempt = marked.attempted, marked_blocked = marked.blocked, marked_sent = marked.sent,
            empty_marked_attempt = empty_marked.attempted, empty_marked_blocked = empty_marked.blocked, empty_marked_sent = empty_marked.sent,
            insert_attempt = ordinary.attempted, insert_blocked = ordinary.blocked, insert_sent = ordinary.sent,
            confirmed_attempt = confirmed.attempted, confirmed_blocked = confirmed.blocked, confirmed_sent = confirmed.sent,
            sent_semantics = "仅请求原有IPC，非宿主确认", "微信草稿诊断");
    });
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use tests::capture_writes;

/// 操作闭包原样执行一次；仅诊断自身隔离 panic，不拦截父类调用的行为。
pub(crate) fn callback<R>(
    name: &'static str,
    current: impl FnOnce() -> bool,
    forward: impl FnOnce() -> R,
) -> R {
    let diagnostic = callback::Callback::begin(name);
    let value = forward();
    diagnostic.finish(current);
    value
}

pub(crate) fn write_marked(empty: bool, outcome: &'static str) {
    write(WriteKind::Marked, outcome);
    if empty {
        write(WriteKind::EmptyMarked, outcome);
    }
}
