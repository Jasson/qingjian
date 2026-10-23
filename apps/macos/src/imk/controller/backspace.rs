//! Terminal 草稿的普通 ⌫ 入口；先核验当前应用，再优先删除待选拼音。

use super::QingjianInputController;
use crate::host;
use crate::imk::{TextClient, secure_input};

impl QingjianInputController {
    pub(super) fn handle_terminal_backspace(&self, client: TextClient<'_>, repeat: bool) -> bool {
        let observation = host::with(|h| h.live_observation_token());
        let Some(observation) = observation else {
            trace_rejection(51, true, repeat, "host_busy");
            return true;
        };
        let application = client.bundle_identifier();
        if !self.is_current_controller() || secure_input::enabled() {
            trace_rejection(51, true, repeat, "controller_or_secure_changed");
            return true;
        }
        let result = host::with(|h| {
            if !observation.matches(&h.live_observation_token()) {
                trace_rejection(51, true, repeat, "input_changed_during_client_read");
                return None;
            }
            if !h.accept_live_application(application.as_deref()) || !h.terminal_draft_active() {
                trace_rejection(51, true, repeat, "not_terminal_draft");
                return None;
            }
            let pinyin_before = h.engine.composition().text().chars().count();
            let draft_before = h.live.draft.committed().chars().count();
            let was_paused = h.live.paused;
            let (composing, changed) = h.backspace_terminal_draft()?;
            let pinyin_after = h.engine.composition().text().chars().count();
            let draft_after = h.live.draft.committed().chars().count();
            if !repeat {
                tracing::info!(target: "live_translation", key_code = 51,
                    branch = if composing { "pinyin" } else { "draft" },
                    pinyin_before, pinyin_after, draft_before, draft_after,
                    was_paused, changed, epoch = h.live.epoch, "草稿退格已处理");
            }
            Some((composing, changed, pinyin_after, draft_after))
        })
        .flatten();
        let Some((composing, changed, remaining, draft_length)) = result else {
            // 已核实转到其他应用时，让该应用正常处理此退格；不操作旧草稿。
            return application
                .as_deref()
                .is_none_or(|app| app == "com.apple.Terminal");
        };
        if composing {
            self.refresh(client);
        } else {
            self.render(client);
        }
        host::with(|h| {
            if !h.terminal_draft_active() {
                return;
            }
            let notice = if composing && !changed {
                "拼音光标前没有可删按键，请查看待选拼音中的光标位置"
            } else if composing && remaining > 0 {
                "已删除一个待选拼音键；选字后可继续删除原文末字"
            } else if composing {
                "待选拼音已删空，继续按 ⌫ 删除原文末字"
            } else if draft_length > 0 {
                "已删除原文末字，译文会随之更新"
            } else {
                "原文草稿已删空"
            };
            h.live_action_notice(notice);
        });
        true
    }
}

pub(super) fn trace_rejection(key: u16, staged: bool, repeat: bool, reason: &'static str) {
    if key == 51 && staged && !repeat {
        tracing::info!(target: "live_translation", key_code = key, reason, "草稿退格未进入编辑");
    }
}
