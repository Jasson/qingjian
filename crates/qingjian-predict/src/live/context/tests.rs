//! 上下文可信度回放：局部窗口保留历史，可信编辑器清空才重置。

use super::ContextGuard;
use crate::live::SentenceTracker;
use crate::live::context::{ContextConfidence, ContextRange, ContextSnapshot};

fn snapshot(before: &str) -> ContextSnapshot {
    let length = before.encode_utf16().count();
    ContextSnapshot {
        client_proxy: 1,
        application: Some("test.editor".to_owned()),
        document_length: length,
        selected: Some(ContextRange {
            start: length,
            length: 0,
        }),
        marked: None,
        before_range: Some(ContextRange { start: 0, length }),
        before: Some(before.to_owned()),
        probe_range: Some(ContextRange {
            start: 0,
            length: 0,
        }),
        probe_length: Some(0),
        lookback_limit: 4096,
    }
}

#[test]
fn initially_empty_successful_probe_does_not_prove_complete_document() {
    let mut guard = ContextGuard::default();
    let result = guard.observe(&snapshot(""), "已经上屏");
    assert_eq!(result.confidence, ContextConfidence::Partial);
    assert!(result.reset_reason.is_none());
}

#[test]
fn partial_empty_composition_keeps_accumulating_whole_sentence() {
    let mut tracker = SentenceTracker::default();
    let mut guard = ContextGuard::default();
    for word in ["我", "正在", "学习", "英语"] {
        let mut context = snapshot("");
        context.document_length = 2;
        context.marked = Some(ContextRange {
            start: 0,
            length: 2,
        });
        context.selected = Some(ContextRange {
            start: 2,
            length: 0,
        });
        context.probe_range = context.marked;
        context.probe_length = Some(2);
        let result = guard.observe(&context, tracker.committed());
        assert_eq!(result.confidence, ContextConfidence::Partial);
        assert!(result.reset_reason.is_none());
        tracker.preview(word);
        tracker.commit(word);
    }
    assert_eq!(tracker.source(), "我正在学习英语");
}

#[test]
fn full_then_real_empty_document_requires_reset() {
    let mut guard = ContextGuard::default();
    assert_eq!(
        guard.observe(&snapshot("你好"), "你好").confidence,
        ContextConfidence::Full
    );
    let result = guard.observe(&snapshot(""), "你好");
    assert_eq!(result.confidence, ContextConfidence::Full);
    assert_eq!(result.reset_reason, Some("trusted_context_changed"));
}

#[test]
fn full_then_local_marked_origin_and_short_window_downgrade_trust() {
    for marked in [false, true] {
        let mut guard = ContextGuard::default();
        guard.observe(&snapshot("你好世界"), "你好世界");
        let mut context = snapshot(if marked { "" } else { "世界" });
        if marked {
            context.document_length = 2;
            context.marked = Some(ContextRange {
                start: 0,
                length: 2,
            });
            context.selected = Some(ContextRange {
                start: 2,
                length: 0,
            });
            context.probe_range = context.marked;
            context.probe_length = Some(2);
        }
        let result = guard.observe(&context, "你好世界");
        assert_eq!(result.confidence, ContextConfidence::Partial);
        assert!(result.reset_reason.is_none());
        assert!(
            guard
                .observe(&snapshot(""), "你好世界")
                .reset_reason
                .is_none()
        );
        assert_eq!(
            guard.observe(&snapshot("你好世界"), "你好世界").confidence,
            ContextConfidence::Full
        );
    }
}

#[test]
fn invalid_utf16_ranges_or_failed_probe_are_unreadable() {
    let mut guard = ContextGuard::default();
    let emoji = snapshot("😀");
    assert_eq!(
        guard.observe(&emoji, "😀").confidence,
        ContextConfidence::Full
    );
    let mut incorrect = emoji.clone();
    incorrect.before = Some("a".to_owned());
    assert_eq!(
        guard.observe(&incorrect, "😀").confidence,
        ContextConfidence::Unreadable
    );
    incorrect = emoji.clone();
    incorrect.probe_length = None;
    assert_eq!(
        guard.observe(&incorrect, "😀").confidence,
        ContextConfidence::Unreadable
    );
    incorrect = emoji;
    incorrect.selected = Some(ContextRange {
        start: usize::MAX,
        length: 1,
    });
    assert_eq!(
        guard.observe(&incorrect, "😀").confidence,
        ContextConfidence::Unreadable
    );
}

#[test]
fn bounded_lookback_still_verifies_full_history_tail_and_proxy_change_is_harmless() {
    let history = "中".repeat(5000);
    let mut context = snapshot(&"中".repeat(4096));
    context.document_length = 5000;
    context.selected = Some(ContextRange {
        start: 5000,
        length: 0,
    });
    context.before_range = Some(ContextRange {
        start: 904,
        length: 4096,
    });
    let mut guard = ContextGuard::default();
    assert_eq!(
        guard.observe(&context, &history).confidence,
        ContextConfidence::Full
    );
    context.client_proxy = 2;
    assert!(guard.observe(&context, &history).reset_reason.is_none());
    guard.reset();
    assert!(
        guard
            .observe(&snapshot(""), &history)
            .reset_reason
            .is_none()
    );
    context.application = Some("other.editor".to_owned());
    assert_eq!(
        guard.observe(&context, &history).reset_reason,
        Some("application_changed")
    );
}

#[test]
fn old_or_unrelated_readable_fragment_cannot_establish_full_trust() {
    let mut guard = ContextGuard::default();
    for context in [snapshot("较长的终端提示符或过时文字"), snapshot("")] {
        let result = guard.observe(&context, "在终端翻译");
        assert_eq!(result.confidence, ContextConfidence::Partial);
        assert!(result.reset_reason.is_none());
    }
    guard.observe(&snapshot("在终端翻译"), "在终端翻译");
    let mut unavailable = snapshot("");
    unavailable.probe_length = None;
    guard.observe(&unavailable, "在终端翻译");
    assert!(
        guard
            .observe(&snapshot("较长的终端提示符或过时文字"), "在终端翻译")
            .reset_reason
            .is_none()
    );
}

#[test]
fn explicit_selection_resets_and_missing_application_does_not_erase_identity() {
    let mut guard = ContextGuard::default();
    let mut context = snapshot("原有文字");
    context.selected = Some(ContextRange {
        start: 0,
        length: 4,
    });
    context.before = Some(String::new());
    context.before_range = Some(ContextRange {
        start: 0,
        length: 0,
    });
    assert_eq!(
        guard.observe(&context, "原有文字").reset_reason,
        Some("selection_changed")
    );
    let mut missing = snapshot("");
    missing.application = None;
    guard.observe(&missing, "原有文字");
    let mut other = snapshot("");
    other.application = Some("different.editor".to_owned());
    assert_eq!(
        guard.observe(&other, "原有文字").reset_reason,
        Some("application_changed")
    );
}

#[test]
fn zero_length_marked_is_not_active_even_at_stale_out_of_bounds_offset() {
    for start in [0, 3, 5, usize::MAX - 1] {
        let mut context = snapshot("你说的都对");
        context.marked = Some(ContextRange { start, length: 0 });
        assert!(context.active_marked().is_none());
        let decision = ContextGuard::default().observe(&context, "你说的都对");
        assert_eq!(decision.confidence, ContextConfidence::Full);
        assert!(decision.reset_reason.is_none());
        assert_eq!(context.marked.unwrap().start, start);
    }
    let mut invalid = snapshot("你说的都对");
    invalid.marked = Some(ContextRange {
        start: 99,
        length: 1,
    });
    assert_eq!(
        ContextGuard::default()
            .observe(&invalid, "你说的都对")
            .confidence,
        ContextConfidence::Unreadable
    );
}

#[test]
fn composing_without_active_marked_does_not_compare_pinyin_against_committed_history() {
    for marked in [
        None,
        Some(ContextRange {
            start: 5,
            length: 0,
        }),
    ] {
        let mut guard = ContextGuard::default();
        guard.observe(&snapshot("你说的都对"), "你说的都对");
        let mut composing = snapshot("你说的都对ni");
        composing.marked = marked;
        let decision = guard.observe_composition(&composing, "你说的都对", true);
        assert_eq!(decision.confidence, ContextConfidence::Partial);
        assert!(decision.reset_reason.is_none());
        assert_eq!(
            guard
                .observe_composition(&snapshot("你说的都对你"), "你说的都对你", false)
                .confidence,
            ContextConfidence::Full
        );
    }
}
