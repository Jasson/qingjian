//! 回放终端异步回显与受控事件边界，不把屏幕缓冲当成编辑文档。

use super::ContextGuard;
use crate::live::SentenceTracker;
use crate::live::context::{ContextConfidence, ContextPolicy, ContextRange, ContextSnapshot};

fn screen(before: &str, caret: usize, document_length: usize) -> ContextSnapshot {
    let length = before.encode_utf16().count();
    ContextSnapshot {
        client_proxy: 1,
        application: Some("test.event-host".to_owned()),
        document_length,
        selected: Some(ContextRange {
            start: caret,
            length: 0,
        }),
        marked: None,
        before_range: Some(ContextRange {
            start: caret - length,
            length,
        }),
        before: Some(before.to_owned()),
        probe_range: Some(ContextRange {
            start: 0,
            length: 1,
        }),
        probe_length: Some(1),
        lookback_limit: length,
    }
}

#[test]
fn delayed_echo_same_caret_and_document_preserves_preview_commit_stream() {
    let mut tracker = SentenceTracker::new();
    let mut guard = ContextGuard::default();
    // 实际日志中的 h2 → h3，光标和文档长度不变，屏幕仍回显上一轮文字。
    tracker.commit("你好");
    let stale = screen(&format!("{}你好", "屏".repeat(4094)), 5658, 5930);
    let first =
        guard.observe_with_policy(&stale, tracker.committed(), false, ContextPolicy::EventOnly);
    assert_eq!(first.confidence, ContextConfidence::Partial);
    tracker.preview("吗");
    assert_eq!(tracker.source(), "你好吗");
    tracker.commit("吗");
    let delayed =
        guard.observe_with_policy(&stale, tracker.committed(), false, ContextPolicy::EventOnly);
    assert!(delayed.reset_reason.is_none());
    assert_eq!(delayed.confidence, ContextConfidence::Partial);
    assert!(delayed.context_mismatch);
    tracker.preview("朋友");
    tracker.commit("朋友");
    assert_eq!(tracker.source(), "你好吗朋友");
    let echoed = screen("你好吗朋友", 5658, 5930);
    assert!(
        guard
            .observe_with_policy(
                &echoed,
                tracker.committed(),
                false,
                ContextPolicy::EventOnly
            )
            .reset_reason
            .is_none()
    );
    assert_eq!(guard.confidence(), ContextConfidence::Partial);
}

#[test]
fn full_screen_and_empty_screen_never_promote_event_host_to_full() {
    let history = "正在写的输入缓冲";
    let mut guard = ContextGuard::default();
    for before in ["界".repeat(4096), history.to_owned(), String::new()] {
        let sample = screen(&before, 5658, 5930);
        let decision = guard.observe_with_policy(&sample, history, false, ContextPolicy::EventOnly);
        assert_eq!(decision.confidence, ContextConfidence::Partial);
        assert!(decision.reset_reason.is_none());
    }
}

#[test]
fn event_host_screen_selection_is_diagnostic_but_application_change_is_hard_boundary() {
    let mut guard = ContextGuard::default();
    let mut sample = screen("你好", 5658, 5930);
    sample.selected.as_mut().unwrap().length = 2;
    assert!(
        guard
            .observe_with_policy(&sample, "你好", false, ContextPolicy::EventOnly)
            .reset_reason
            .is_none()
    );
    sample.application = None;
    assert!(
        guard
            .observe_with_policy(&sample, "你好", false, ContextPolicy::EventOnly)
            .reset_reason
            .is_none()
    );
    sample.application = Some("test.browser".to_owned());
    assert_eq!(
        guard
            .observe_with_policy(&sample, "你好", false, ContextPolicy::VerifiedDocument)
            .reset_reason,
        Some("application_changed")
    );
}

#[test]
fn browser_keeps_verified_mismatch_and_event_boundaries_start_fresh() {
    let mut guard = ContextGuard::default();
    let mut tracker = SentenceTracker::new();
    tracker.commit("你好");
    let sample = screen("你好", 2, 2);
    assert_eq!(
        guard.observe(&sample, tracker.committed()).confidence,
        ContextConfidence::Full
    );
    tracker.commit("吗");
    assert_eq!(
        guard.observe(&sample, tracker.committed()).reset_reason,
        None
    );
    // 覆盖全部历史的旧屏幕在已建立的完整能力下仍识别为外部编辑。
    let changed = screen("旧文字", 3, 3);
    guard.observe(&screen("你好吗", 3, 3), tracker.committed());
    assert_eq!(
        guard.observe(&changed, tracker.committed()).reset_reason,
        Some("trusted_context_changed")
    );
    // 平台明确边界最终调用 pause/reset；下一段不能复用旧 history 或候选。
    tracker.preview("未提交候选");
    tracker.reset();
    guard.reset();
    tracker.commit("新一段");
    assert_eq!(tracker.source(), "新一段");
    tracker.commit("\n下一句");
    assert_eq!(tracker.source(), "下一句");
    tracker.backspace();
    assert_eq!(tracker.source(), "下一");
}
