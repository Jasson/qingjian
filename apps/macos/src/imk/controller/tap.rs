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
    if crate::imk::editor::session_owned() {
        return Some(false);
    }
    let controller = active_controller()?;
    if active_controller_id() != Some(batch.owner)
        || secure_input::enabled()
        || backspace_tap::frontmost_pid() != Some(batch.pid)
        || !backspace_tap::current(batch.generation, batch.serial)
    {
        return Some(false);
    }
    let guard = OutputGuard::Tap(batch.generation, batch.serial, &batch.token);
    if matches!(batch.action, Action::InsertOriginal | Action::PasteDraft) {
        let client: Option<Retained<AnyObject>> = unsafe { msg_send![&*controller, client] };
        let Some(client) = client else {
            return Some(false);
        };
        if crate::imk::editor::session_owned()
            || !controller.is_current_controller()
            || secure_input::enabled()
            || backspace_tap::frontmost_pid() != Some(batch.pid)
        {
            return Some(false);
        }
        return if batch.action == Action::PasteDraft {
            controller.paste_draft(TextClient::new(&client), Some(guard))
        } else {
            controller.output_tapped_original(TextClient::new(&client), guard)
        };
    }
    host::with(|h| {
        if !h.draft_active() || !guard.valid(h) {
            return false;
        }
        match batch.action {
            Action::Backspace => h.backspace_draft_batch(batch.count),
            Action::CancelDraft => {
                h.cancel_draft("draft_shortcut_cancelled");
                backspace_tap::action_consumed(batch.generation, batch.serial);
            }
            Action::PasteConflict => {
                backspace_tap::action_consumed(batch.generation, batch.serial);
                h.live_action_notice(
                    "粘贴草稿快捷键与已有动作冲突，未粘贴或输出；请修改快捷键设置",
                );
            }
            Action::InsertOriginal | Action::PasteDraft => unreachable!(),
        }
        true
    })
}
