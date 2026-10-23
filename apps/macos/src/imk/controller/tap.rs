//! 主线程执行已绑定草稿与配置的 tap 意图；所有客户 IPC 均在 Host 借用外。

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;

use super::click::{active_controller, active_controller_id};
use super::output_guard::OutputGuard;
use crate::host::{self, ObservationToken};
use crate::imk::{TextClient, backspace_tap, secure_input};
use backspace_tap::{Action, Batch};

pub(crate) fn dispatch_draft_tap(batch: &Batch<ObservationToken>) -> Option<bool> {
    let controller = active_controller()?;
    if active_controller_id() != Some(batch.owner)
        || secure_input::enabled()
        || backspace_tap::frontmost_pid() != Some(batch.pid)
        || !backspace_tap::current(batch.generation, batch.serial)
    {
        return Some(false);
    }
    let guard = OutputGuard::Tap(batch.generation, batch.serial, &batch.token);
    if batch.action == Action::InsertOriginal {
        let client: Option<Retained<AnyObject>> = unsafe { msg_send![&*controller, client] };
        let Some(client) = client else {
            return Some(false);
        };
        if !controller.is_current_controller()
            || secure_input::enabled()
            || backspace_tap::frontmost_pid() != Some(batch.pid)
        {
            return Some(false);
        }
        return controller.output_tapped_original(TextClient::new(&client), guard);
    }
    host::with(|h| {
        if !h.terminal_draft_active() || !guard.valid(h) {
            return false;
        }
        match batch.action {
            Action::Backspace => h.backspace_terminal_draft_batch(batch.count),
            Action::CancelDraft => {
                h.cancel_terminal_draft("draft_shortcut_cancelled");
                backspace_tap::action_consumed(batch.generation, batch.serial);
            }
            Action::InsertOriginal => unreachable!(),
        }
        true
    })
}
