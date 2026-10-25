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
            (h.can_restore_draft() && expected.is_none_or(|guard| guard.valid(h)))
                .then(|| (h.live_observation_token(), h.live.raw_generation))
        })
        .flatten();
        let Some((observation, raw_generation)) = state else {
            return;
        };
        let application = client.bundle_identifier();
        let foreground = frontmost::matches(observation.application());
        if !foreground
            || application.as_deref() != observation.application()
            || !self.is_current_controller()
            || secure_input::enabled()
        {
            return;
        }
        let restored = host::with(|h| {
            if !h.draft_active()
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
        let still_current = foreground
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
