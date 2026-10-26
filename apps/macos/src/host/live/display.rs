//! 实时翻译条的显示状态；更新期间保留最近一次成功译文。

use super::receipt::SuccessReceipt;

#[derive(Default)]
pub(super) struct DisplayState {
    successful: Option<SuccessReceipt>,

    pending: bool,

    error: Option<String>,
}

impl DisplayState {
    pub fn begin(&mut self, unavailable: Option<&str>) {
        self.pending = unavailable.is_none();
        self.error = unavailable.map(str::to_owned);
    }

    pub fn complete(&mut self, result: Result<SuccessReceipt, String>) {
        self.pending = false;
        match result {
            Ok(text) => {
                self.successful = Some(text);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn copy_text(&self) -> Option<&str> {
        self.successful
            .as_ref()
            .map(|receipt| receipt.text.as_str())
    }

    /// 只复用当前实际显示的那条成功译文；不同会话不能取出。
    pub fn cached_receipt(&self, epoch: u64) -> Option<&SuccessReceipt> {
        self.successful
            .as_ref()
            .filter(|receipt| receipt.epoch == epoch)
    }

    pub fn current_receipt(
        &self,
        epoch: u64,
        source: &str,
        request_id: Option<u64>,
    ) -> Option<&SuccessReceipt> {
        self.cached_receipt(epoch).filter(|receipt| {
            !self.pending
                && self.error.is_none()
                && receipt.source == source
                && Some(receipt.request_id) == request_id
        })
    }

    pub fn pause(&mut self) {
        self.pending = false;
        self.error = None;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn presentation(&self) -> (&str, &str) {
        match (&self.successful, self.pending, &self.error) {
            (Some(receipt), true, _) => (&receipt.text, "本地实时翻译 · 更新中…"),
            (Some(receipt), false, Some(_)) => {
                (&receipt.text, "本地实时翻译 · 更新失败，显示上次译文")
            }
            (Some(receipt), false, None) => (&receipt.text, "本地实时翻译 · 预译会随输入修正"),
            (None, _, Some(error)) => (error, "本地翻译暂不可用"),
            (None, _, None) => ("翻译中…", "本地实时翻译"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DisplayState;
    use crate::host::live::receipt::SuccessReceipt;

    fn receipt(source: &str, text: &str, request_id: u64) -> SuccessReceipt {
        SuccessReceipt {
            source: source.to_owned(),
            text: text.to_owned(),
            request_id,
            epoch: 2,
        }
    }

    #[test]
    fn current_receipt_rejects_pending_failed_other_request_or_epoch_even_after_source_returns() {
        let mut display = DisplayState::default();
        display.complete(Ok(receipt("A", "first", 1)));
        assert!(display.current_receipt(2, "A", Some(1)).is_some());
        display.begin(None);
        assert!(display.current_receipt(2, "A", Some(1)).is_none());
        display.complete(Err("failed".to_owned()));
        assert!(display.current_receipt(2, "A", Some(1)).is_none());
        display.complete(Ok(receipt("A", "first", 1)));
        assert!(display.current_receipt(2, "B", Some(2)).is_none());
        assert!(display.current_receipt(2, "A", Some(3)).is_none());
        assert!(display.current_receipt(3, "A", Some(1)).is_none());
    }

    #[test]
    fn pause_preserves_exactly_the_visible_success_and_cancels_pending_presentation() {
        let mut display = DisplayState::default();
        display.begin(None);
        assert!(display.cached_receipt(2).is_none());
        display.complete(Ok(receipt("原句", "Translation", 1)));
        display.begin(None);
        assert_eq!(display.presentation().0, "Translation");
        display.pause();
        assert_eq!(display.cached_receipt(2).unwrap().source, "原句");
        assert_eq!(display.cached_receipt(2).unwrap().request_id, 1);
        assert_eq!(display.presentation().0, "Translation");
        assert!(!display.presentation().1.contains("更新中"));
        assert!(display.cached_receipt(3).is_none());
    }

    #[test]
    fn new_success_replaces_cache_and_hard_reset_removes_display_and_receipt() {
        let mut display = DisplayState::default();
        display.complete(Ok(receipt("原句", "First", 1)));
        display.pause();
        display.begin(None);
        display.complete(Ok(receipt("新句", "Second", 2)));
        assert_eq!(display.presentation().0, "Second");
        assert_eq!(display.cached_receipt(2).unwrap().source, "新句");
        display.reset();
        assert!(display.copy_text().is_none());
        assert!(display.cached_receipt(2).is_none());
    }

    #[test]
    fn failed_update_keeps_visible_cached_success_and_copy_is_not_truncated() {
        let mut display = DisplayState::default();
        let text = "完整译文".repeat(300);
        display.complete(Ok(receipt("原句", &text, 1)));
        display.begin(None);
        display.complete(Err("请求失败".to_owned()));
        assert_eq!(display.copy_text(), Some(text.as_str()));
        assert_eq!(display.cached_receipt(2).unwrap().source, "原句");
        display.pause();
        assert_eq!(display.presentation().0, text);
    }
}
