//! 空 marked 范围的宿主兼容回归；范围为空不能截断已上屏句子。

use objc2_foundation::NSRange;
use qingjian_predict::live::SentenceTracker;
use qingjian_predict::live::context::{ContextConfidence, ContextGuard};

use super::context::ContextClient;
use super::no_marked;
use crate::imk::TextClient;

#[test]
fn zero_length_marked_and_not_found_read_full_caret_context_for_exact_replacement() {
    for marked in [
        NSRange::new(3, 0),
        NSRange::new(5, 0),
        NSRange::new(99, 0),
        no_marked(),
    ] {
        let mock = ContextClient::new(Some("你说的都对"), 5, NSRange::new(5, 0), marked);
        let snapshot = TextClient::new(&mock).live_context(4096);
        assert_eq!(snapshot.before.as_deref(), Some("你说的都对"));
        assert!(snapshot.active_marked().is_none());
        assert_eq!(mock.requests(), [NSRange::new(0, 5)]);
        assert_eq!(
            ContextGuard::default()
                .observe(&snapshot, "你说的都对")
                .confidence,
            ContextConfidence::Full
        );
        let mut tracker = SentenceTracker::default();
        tracker.commit("你说的都对");
        let plan = tracker
            .replacement_plan(
                "你说的都对",
                snapshot.before.as_deref().unwrap(),
                5,
                0,
                snapshot.active_marked().is_some(),
            )
            .expect("零长度不阻止精确替换");
        assert_eq!(plan.location_utf16, 0);
        assert_eq!(plan.length_utf16, 5);
        assert!(
            plan.matches_original(
                &TextClient::new(&mock)
                    .read_range(NSRange::new(0, 5))
                    .unwrap()
            )
        );
    }
}

#[test]
fn nonempty_marked_remains_active_and_invalid_ranges_are_not_read() {
    let mock = ContextClient::new(Some("你说的do"), 5, NSRange::new(5, 0), NSRange::new(3, 2));
    let snapshot = TextClient::new(&mock).live_context(4096);
    assert_eq!(snapshot.before.as_deref(), Some("你说的"));
    assert!(snapshot.active_marked().is_some());
    let mut tracker = SentenceTracker::default();
    tracker.commit("你说的");
    assert!(
        tracker
            .replacement_plan(
                "你说的",
                snapshot.before.as_deref().unwrap(),
                5,
                0,
                snapshot.active_marked().is_some()
            )
            .is_none()
    );
    let invalid = ContextClient::new(
        Some("你说的都对"),
        5,
        NSRange::new(5, 0),
        NSRange::new(99, 1),
    );
    let snapshot = TextClient::new(&invalid).live_context(4096);
    assert!(snapshot.before.is_none());
    assert!(invalid.requests().is_empty());
    assert_eq!(
        ContextGuard::default()
            .observe(&snapshot, "你说的都对")
            .confidence,
        ContextConfidence::Unreadable
    );
}

#[test]
fn engine_composing_with_zero_marked_preserves_history_until_confirmed_commit() {
    let mut guard = ContextGuard::default();
    let full = ContextClient::new(Some("你说的都对"), 5, NSRange::new(5, 0), no_marked());
    guard.observe(&TextClient::new(&full).live_context(4096), "你说的都对");
    let mut tracker = SentenceTracker::default();
    tracker.commit("你说的都对");
    let composing = ContextClient::new(
        Some("你说的都对ni"),
        7,
        NSRange::new(7, 0),
        NSRange::new(5, 0),
    );
    let snapshot = TextClient::new(&composing).live_context(4096);
    let decision = guard.observe_composition(&snapshot, tracker.committed(), true);
    assert_eq!(decision.confidence, ContextConfidence::Partial);
    assert!(decision.reset_reason.is_none());
    tracker.preview("你");
    assert_eq!(tracker.source(), "你说的都对你");
    assert!(
        tracker
            .replacement_plan(
                "你说的都对你",
                snapshot.before.as_deref().unwrap(),
                7,
                0,
                false
            )
            .is_none()
    );
    tracker.commit("你");
    let committed = ContextClient::new(
        Some("你说的都对你"),
        6,
        NSRange::new(6, 0),
        NSRange::new(6, 0),
    );
    assert_eq!(
        guard
            .observe_composition(
                &TextClient::new(&committed).live_context(4096),
                tracker.committed(),
                false
            )
            .confidence,
        ContextConfidence::Full
    );
}
