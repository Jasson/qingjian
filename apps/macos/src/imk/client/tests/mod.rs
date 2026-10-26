//! IMK 文本写入与上下文读取证据回归。

mod context;
mod draft;
mod exact;
mod marked;
mod selected;
mod selection;
mod state;

use super::TextClient;
use context::ContextClient;
use objc2::DeclaredClass;
use objc2_foundation::{NSNotFound, NSRange};
use qingjian_predict::live::{
    SentenceTracker,
    context::{ContextConfidence, ContextGuard},
};
use selection::SelectionClient;

fn no_marked() -> NSRange {
    NSRange::new(NSNotFound as usize, 0)
}

#[test]
fn empty_mode_switch_never_writes_to_selected_application_text() {
    let mock = SelectionClient::new();
    TextClient::new(&mock).commit_pending_raw("");
    assert!(mock.ivars().borrow().is_empty());
}

#[test]
fn mode_switch_commits_raw_once_without_followup_marked_text_write() {
    let mock = SelectionClient::new();
    TextClient::new(&mock).commit_pending_raw("nihao");
    assert_eq!(&*mock.ivars().borrow(), &["insert:nihao"]);
}

#[test]
fn empty_context_requires_readability_and_previously_verified_full_history() {
    let mut guard = ContextGuard::default();
    let full = ContextClient::new(Some("你好😀"), 4, NSRange::new(4, 0), no_marked());
    let snapshot = TextClient::new(&full).live_context(4096);
    assert_eq!(snapshot.probe_length, Some(4));
    assert_eq!(full.requests().len(), 1);
    assert_eq!(
        guard.observe(&snapshot, "你好😀").confidence,
        ContextConfidence::Full
    );
    let empty = ContextClient::new(Some(""), 0, NSRange::new(0, 0), no_marked());
    let snapshot = TextClient::new(&empty).live_context(4096);
    assert_eq!(snapshot.probe_length, Some(0));
    assert_eq!(empty.requests().len(), 1);
    assert_eq!(
        guard.observe(&snapshot, "你好😀").reset_reason,
        Some("trusted_context_changed")
    );
}

#[test]
fn unreadable_zero_context_is_not_confirmed_empty_and_invalid_ranges_are_not_read() {
    let mock = ContextClient::new(None, 0, NSRange::new(0, 0), no_marked());
    let snapshot = TextClient::new(&mock).live_context(4096);
    assert!(snapshot.before.is_none());
    assert!(snapshot.probe_length.is_none());
    let mut guard = ContextGuard::default();
    let full = ContextClient::new(Some("正在输入"), 4, NSRange::new(4, 0), no_marked());
    assert_eq!(
        guard
            .observe(&TextClient::new(&full).live_context(4096), "正在输入")
            .confidence,
        ContextConfidence::Full
    );
    let decision = guard.observe(&snapshot, "正在输入");
    assert_eq!(decision.confidence, ContextConfidence::Unreadable);
    assert!(decision.reset_reason.is_none());
    let invalid = ContextClient::new(Some(""), 0, NSRange::new(10, 0), no_marked());
    let snapshot = TextClient::new(&invalid).live_context(4096);
    assert!(snapshot.before_range.is_none());
    assert!(invalid.requests().is_empty());
}

#[test]
fn marked_only_window_keeps_preceding_commits_in_whole_sentence() {
    let local = ContextClient::new(Some("xy"), 2, NSRange::new(2, 0), NSRange::new(0, 2));
    let mut guard = ContextGuard::default();
    let mut tracker = SentenceTracker::default();
    for text in ["我", "正在", "学习", "英语"] {
        let snapshot = TextClient::new(&local).live_context(4096);
        let decision = guard.observe(&snapshot, tracker.committed());
        assert_eq!(decision.confidence, ContextConfidence::Partial);
        assert!(decision.reset_reason.is_none());
        tracker.preview(text);
        tracker.commit(text);
    }
    assert_eq!(tracker.source(), "我正在学习英语");
}

#[test]
fn exact_range_readback_and_single_replacement_use_utf16_and_preserve_spaces() {
    let mock = ContextClient::new(Some("开头。😀你好  "), 9, NSRange::new(9, 0), no_marked());
    let client = TextClient::new(&mock);
    let mut tracker = SentenceTracker::default();
    tracker.commit("😀你好  ");
    let plan = tracker
        .replacement_plan("😀你好", "开头。😀你好  ", 9, 0, false)
        .expect("精确原句");
    let range = NSRange::new(plan.location_utf16, plan.length_utf16);
    assert_eq!(range, NSRange::new(3, 6));
    let actual = client.read_range(range).expect("应用可读取范围");
    assert!(plan.matches_original(&actual));
    client.replace_range(&plan.replacement("Hello"), range);
    assert_eq!(mock.writes(), vec![("Hello  ".to_owned(), range)]);
}

#[test]
fn changed_or_unreadable_original_does_not_pass_exact_range_gate() {
    let changed = ContextClient::new(Some("另文"), 2, NSRange::new(2, 0), no_marked());
    let mut tracker = SentenceTracker::default();
    tracker.commit("原文");
    let plan = tracker
        .replacement_plan("原文", "原文", 2, 0, false)
        .expect("初始原句");
    let actual = TextClient::new(&changed)
        .read_range(NSRange::new(0, 2))
        .expect("可读但已改变");
    assert!(!plan.matches_original(&actual));
    assert!(changed.writes().is_empty());
    let unsupported = ContextClient::new(None, 2, NSRange::new(2, 0), no_marked());
    assert!(
        TextClient::new(&unsupported)
            .read_range(NSRange::new(0, 2))
            .is_none()
    );
    assert!(unsupported.writes().is_empty());
    let invalid = ContextClient::new(Some("原文"), 2, NSRange::new(2, 0), no_marked());
    assert!(
        TextClient::new(&invalid)
            .read_range(NSRange::new(usize::MAX - 1, 5))
            .is_none()
    );
    assert!(invalid.requests().is_empty());
}
