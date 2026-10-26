//! IMK 后备入口也排入同一草稿动作队列，不能绕过按住周期隔离。

use objc2_app_kit::NSEvent;

use super::QingjianInputController;
use crate::imk::{TextClient, backspace_tap, secure_input};

impl QingjianInputController {
    pub(super) fn handle_draft_shortcut(
        &self,
        event: &NSEvent,
        _client: TextClient<'_>,
    ) -> Option<bool> {
        if !self.is_current_controller() || secure_input::enabled() {
            return None;
        }
        backspace_tap::queue_imk_action(
            i64::from(event.keyCode()),
            event.modifierFlags().bits() as u64,
            event.isARepeat(),
            event.timestamp(),
        )
    }
}
