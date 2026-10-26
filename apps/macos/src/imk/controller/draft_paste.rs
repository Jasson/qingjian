//! 草稿快捷键只追加已核验的纯文本，不经过拼音按键路径。

use super::{QingjianInputController, output_guard::OutputGuard};
use crate::host;
use crate::imk::{TextClient, editor, frontmost, secure_input};
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};

impl QingjianInputController {
    pub(super) fn paste_draft(
        &self,
        client: TextClient<'_>,
        expected: Option<OutputGuard<'_>>,
    ) -> Option<bool> {
        let observation = host::with(|h| {
            (h.draft_active() && expected.is_none_or(|guard| guard.valid(h)))
                .then(|| h.live_observation_token())
        })?;
        let Some(observation) = observation else {
            return Some(false);
        };
        let current = || {
            self.is_current_controller()
                && !editor::session_owned()
                && !secure_input::enabled()
                && frontmost::matches(observation.application())
        };
        if !current() || client.bundle_identifier().as_deref() != observation.application() {
            return Some(false);
        }
        let text = NSPasteboard::generalPasteboard()
            .stringForType(unsafe { NSPasteboardTypeString })
            .map(|value| value.to_string());
        if !current() {
            return Some(false);
        }
        host::with(|h| {
            if !h.draft_active()
                || h.translation.is_some()
                || h.engine.symbol_panel().is_some()
                || h.settings.config().shortcut.paste_draft_conflicts()
                || !observation.matches(&h.live_observation_token())
                || expected.is_some_and(|guard| !guard.valid(h))
            {
                return false;
            }
            // 消费本次意图及旧排队输出，拒绝粘贴也隔离按住周期，不能漏给应用。
            if let Some(OutputGuard::Tap(generation, serial, _)) = expected {
                crate::imk::backspace_tap::action_consumed(generation, serial);
            }
            if !h.engine.composition().is_empty() {
                h.live_action_notice("请先完成拼音选字，再粘贴文本");
                return true;
            }
            let Some(text) = text.filter(|text| !text.is_empty()) else {
                h.live_action_notice("剪贴板没有可粘贴的纯文本");
                return true;
            };
            if qingjian_predict::live::normalize_terminal_output(&text).is_none() {
                h.live_action_notice("文本含不支持的控制字符，未粘贴；换行可转换为空格");
                return true;
            }
            h.paste_live_draft(&text);
            true
        })
    }
}
