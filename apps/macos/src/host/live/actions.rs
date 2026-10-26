//! 实时翻译动作的会话门禁与展示状态；不直接操作应用文本。

use qingjian_predict::live::context::ContextSnapshot;
use qingjian_predict::live::{MAX_REPLACEMENT_UNITS, ReplacementCandidate};

use super::SuccessReceipt;
use super::application::replacement_application_blocker;
use crate::host::Host;

impl Host {
    pub fn accept_live_application(&mut self, application: Option<&str>) -> bool {
        let application = application.filter(|app| !app.trim().is_empty());
        if self.engine.application() != application {
            if application == Some("com.apple.loginwindow") || crate::imk::secure_input::enabled() {
                self.live.archive.clear();
                self.cancel_draft("secure_input");
            } else {
                self.suspend_draft("application_changed");
            }
            self.engine.clear();
            self.reset_session(None, Vec::new());
            self.engine.set_application(application.map(str::to_owned));
            return false;
        }
        true
    }

    pub fn live_terminal_blocked(&self, application: Option<&str>) -> bool {
        application == Some("com.apple.Terminal")
            || self.engine.application() == Some("com.apple.Terminal")
    }

    pub fn live_application_blocker(&self, application: Option<&str>) -> Option<&'static str> {
        replacement_application_blocker(application, self.engine.application())
    }

    pub fn live_copy_text(&self) -> Option<String> {
        self.live
            .active()
            .then(|| self.live.display.copy_text().map(str::to_owned))
            .flatten()
    }

    pub fn live_replacement_receipt(&self) -> Option<SuccessReceipt> {
        if !self.live.active()
            || !self.settings.config().live_translation.allow_replace
            || !self.engine.composition().is_empty()
        {
            return None;
        }
        self.live.display.cached_receipt(self.live.epoch).cloned()
    }

    pub fn live_receipt_is_current(&self, receipt: &SuccessReceipt) -> bool {
        self.live_replacement_receipt().as_ref() == Some(receipt)
    }

    pub fn live_replacement_candidate(
        &self,
        receipt: &SuccessReceipt,
        snapshot: &ContextSnapshot,
    ) -> Option<ReplacementCandidate> {
        if !self.live_receipt_is_current(receipt) {
            return None;
        }
        let selected = snapshot
            .selected
            .filter(|range| range.start < isize::MAX as usize)?;
        ReplacementCandidate::for_selection(
            &receipt.source,
            Some(selected),
            snapshot.active_marked().is_some(),
        )
    }

    pub fn live_replacement_blocker(&self) -> &'static str {
        if !self.engine.composition().is_empty() {
            "active_composition"
        } else if !self.settings.config().live_translation.allow_replace {
            "replacement_disabled"
        } else if self.live_replacement_receipt().is_none() {
            "translation_not_current"
        } else if let Some(snapshot) = self.live.last_context.as_ref() {
            if snapshot.active_marked().is_some() {
                "active_marked"
            } else if snapshot.selected.is_none() {
                "selection_missing"
            } else if snapshot.selected.is_some_and(|range| range.length == 0) {
                "selection_empty"
            } else if self
                .live
                .display
                .cached_receipt(self.live.epoch)
                .is_some_and(|receipt| {
                    receipt.source.encode_utf16().count() > MAX_REPLACEMENT_UNITS
                })
            {
                "sentence_too_long"
            } else {
                "source_or_range_mismatch"
            }
        } else {
            "context_missing"
        }
    }

    /// 仅用户明确执行替换却被拒绝时记录，正常按键与绘制不刷日志。
    pub fn reject_live_replacement(
        &mut self,
        reason: &'static str,
        notice: &str,
        read_attempted: bool,
        exact_read_units: Option<usize>,
        reader: &'static str,
    ) {
        let snapshot = self.live.last_context.as_ref();
        tracing::info!(target: "live_translation", action = "replace", reason,
            request_id = self.live.request_id, epoch = self.live.epoch,
            source_units = self.live.source.encode_utf16().count(), read_attempted, exact_read_units, reader,
            document_units = snapshot.map(|context| context.document_length),
            before_range = ?snapshot.and_then(|context| context.before_range),
            application = snapshot.and_then(|context| context.application.as_deref()),
            history_units = self.live.tracker.committed().encode_utf16().count(),
            selected = ?snapshot.and_then(|context| context.selected),
            marked = ?snapshot.and_then(|context| context.marked),
            active_marked = ?snapshot.and_then(ContextSnapshot::active_marked),
            confidence = ?self.live.context.confidence(),
            "实时翻译替换未执行");
        self.live_action_notice(notice);
    }

    pub fn refresh_live_actions(&mut self) {
        self.render_live_translation();
    }

    pub fn live_action_notice(&mut self, notice: &str) {
        self.live.action_notice = Some(notice.to_owned());
        self.render_live_translation();
    }

    pub(super) fn live_replace_enabled(&self) -> bool {
        !self.live_terminal_blocked(None) && self.live_replacement_receipt().is_some()
    }

    pub(super) fn live_replace_hint(&self) -> Option<&'static str> {
        if self.draft_active() {
            return Some("原文暂存青简 · 确认输出仅填入应用，不发送回车");
        }
        if !self.settings.config().live_translation.allow_replace {
            Some("替换未开启")
        } else if !self.engine.composition().is_empty() {
            Some("先选字上屏再替换")
        } else if self.live_terminal_blocked(None) {
            Some("终端仅支持复制译文")
        } else if self.live_replacement_receipt().is_none() {
            Some("先输入文字并等待成功译文")
        } else if !self.live_replace_enabled() {
            Some("请先选中与当前译文对应的整段原文")
        } else {
            None
        }
    }
}
