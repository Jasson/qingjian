//! 终端草稿模式的开关与确认输出门禁；未确认文字只留在输入法内部。

use crate::candidates::Preedit;
use crate::host::Host;

impl Host {
    /// 系统 tap 的同一输入凭据批次一次编辑、一次查询与刷新，不写 native 原文。
    pub fn backspace_terminal_draft_batch(&mut self, count: u16) {
        if !self.terminal_draft_active() || count == 0 {
            return;
        }
        self.live.resume();
        let changed =
            super::editing::backspace_batch(&mut self.engine, &mut self.live.draft, count);
        self.cancel_prediction();
        let mut preedit = Preedit::plain(
            self.engine.composition().text(),
            self.engine.composition().cursor(),
        );
        let candidates = self
            .engine
            .query()
            .map(|mut query| {
                self.engine.annotate(&mut query.candidates);
                preedit = Preedit::from_marked(&query.marked_segments(), query.segments_cursor());
                query.candidates.items
            })
            .unwrap_or_default();
        self.reset_session(preedit, candidates);
        self.schedule_rescoring();
        self.render();
        self.live_action_notice(if !changed && !self.engine.composition().is_empty() {
            "拼音光标前没有可删按键，请查看待选拼音中的光标位置"
        } else if self.engine.composition().is_empty() && self.live.draft.source().is_empty() {
            "原文草稿已删空；松开 ⌫ 后，下次退格正常交给终端"
        } else {
            "已删除内部草稿，译文会随之更新"
        });
    }

    /// 返回是否编辑拼音、是否实际删除；恢复编辑后再决定刷新范围。
    pub fn backspace_terminal_draft(&mut self) -> Option<(bool, bool)> {
        if !self.terminal_draft_active() {
            return None;
        }
        self.live.resume();
        let (composing, changed) =
            super::editing::backspace(&mut self.engine, &mut self.live.draft);
        if !composing {
            self.submit_live_translation();
        }
        Some((composing, changed))
    }

    pub fn terminal_draft_active(&self) -> bool {
        self.live.active()
            && self.settings.config().live_translation.terminal_draft
            && self.engine.application() == Some("com.apple.Terminal")
    }

    pub fn terminal_draft_output(&self, original: bool) -> Option<String> {
        if !self.terminal_draft_active() {
            return None;
        }
        if original {
            return self
                .live
                .draft
                .original_output_with_pending(&self.engine.raw_preedit().text);
        }
        if !self.engine.composition().is_empty() {
            return None;
        }
        let receipt = self.live.display.current_receipt(
            self.live.epoch,
            &self.live.draft.source(),
            self.live.request_id,
        )?;
        self.live
            .draft
            .translation_output(&receipt.source, &receipt.text, true)
    }

    pub fn cancel_terminal_draft(&mut self, reason: &'static str) {
        self.engine.clear();
        self.cancel_prediction();
        self.reset_session(None, Vec::new());
        self.window.hide();
        self.reset_live_translation(reason);
    }
}
