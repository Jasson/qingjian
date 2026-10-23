//! 仅凭有效 caret 尝试短范围读取；实际文字与写能力必须分别核验。

use super::{context::ContextClient, no_marked, selection::SelectionClient};
use crate::imk::TextClient;
use objc2::DeclaredClass;
use objc2_foundation::{NSNotFound, NSRange};
use qingjian_predict::live::SentenceTracker;
use std::cell::Cell;

#[test]
fn short_read_succeeds_when_document_length_is_zero_or_big_before_is_unreadable() {
    for document_length in [0, 8] {
        let mock = ContextClient::new(
            Some("前句。你说的都对"),
            document_length,
            NSRange::new(8, 0),
            no_marked(),
        );
        mock.ivars().maximum_read_units.set(5);
        let client = TextClient::new(&mock);
        assert!(client.live_context(4096).before.is_none());
        assert!(client.supports_document_access(|| true));
        let metadata = client.live_selection_metadata(|| true).unwrap();
        let mut tracker = SentenceTracker::default();
        tracker.commit("你说的都对");
        let candidate = tracker
            .replacement_candidate(
                "你说的都对",
                metadata.selected,
                metadata.active_marked().is_some(),
            )
            .unwrap();
        let first = client.read_caret_bounded_candidate(&candidate, &metadata, || true);
        assert_eq!(first.reader, "legacy");
        assert!(
            candidate
                .verify_exact(first.text.as_deref().unwrap())
                .is_some()
        );
        let second = client.read_caret_bounded_candidate(&candidate, &metadata, || true);
        let plan = candidate
            .verify_exact(second.text.as_deref().unwrap())
            .unwrap();
        assert_eq!((plan.location_utf16, plan.length_utf16), (3, 5));
        assert!(mock.writes().is_empty());
    }
}

#[test]
fn imk_fallback_requires_exact_actual_range_and_current_state() {
    let mock = ContextClient::new(Some("你说的都对"), 0, NSRange::new(5, 0), no_marked());
    mock.ivars().legacy_readable.set(false);
    let client = TextClient::new(&mock);
    let metadata = client.live_selection_metadata(|| true).unwrap();
    let mut tracker = SentenceTracker::default();
    tracker.commit("你说的都对");
    let candidate = tracker
        .replacement_candidate("你说的都对", metadata.selected, false)
        .unwrap();
    let read = client.read_caret_bounded_candidate(&candidate, &metadata, || true);
    assert_eq!(read.reader, "imk");
    assert!(
        candidate
            .verify_exact(read.text.as_deref().unwrap())
            .is_some()
    );
    mock.ivars().adjust_actual.set(true);
    assert!(
        client
            .read_caret_bounded_candidate(&candidate, &metadata, || true)
            .text
            .is_none()
    );
    let before = mock.requests().len();
    assert!(
        !client
            .read_caret_bounded_candidate(&candidate, &metadata, || false)
            .attempted
    );
    assert_eq!(mock.requests().len(), before);
    let calls = Cell::new(0);
    let before_guarded = mock.requests().len();
    let interrupted = client.read_caret_bounded_candidate(&candidate, &metadata, || {
        calls.set(calls.get() + 1);
        calls.get() < 2
    });
    assert!(interrupted.text.is_none());
    assert_eq!(interrupted.reader, "legacy");
    assert_eq!(mock.requests().len(), before_guarded + 1);
    assert!(client.supports_document_access(|| true));
    mock.ivars().document_access.set(false);
    assert!(!client.supports_document_access(|| true));
    assert!(mock.writes().is_empty());
}

#[test]
fn unsupported_document_access_or_missing_property_selector_cannot_authorize_write() {
    let mock = ContextClient::new(Some("原文"), 0, NSRange::new(2, 0), no_marked());
    mock.ivars().document_access.set(false);
    assert!(!TextClient::new(&mock).supports_document_access(|| true));
    assert_eq!(
        &*mock.ivars().support_queries.borrow(),
        &[u32::from_be_bytes(*b"dapy")]
    );
    assert!(mock.requests().is_empty());
    assert!(mock.writes().is_empty());
    let missing = SelectionClient::new();
    assert!(!TextClient::new(&missing).supports_document_access(|| true));
    assert!(missing.ivars().borrow().is_empty());
}

#[test]
fn metadata_and_read_stop_on_reentry_and_reject_wrong_proxy_or_invalid_caret() {
    let mock = ContextClient::new(Some("原文"), 0, NSRange::new(2, 0), no_marked());
    let client = TextClient::new(&mock);
    let calls = Cell::new(0);
    assert!(
        client
            .live_selection_metadata(|| {
                calls.set(calls.get() + 1);
                calls.get() < 3
            })
            .is_none()
    );
    let mut metadata = client.live_selection_metadata(|| true).unwrap();
    let mut tracker = SentenceTracker::default();
    tracker.commit("原文");
    let candidate = tracker
        .replacement_candidate("原文", metadata.selected, false)
        .unwrap();
    metadata.client_proxy += 1;
    assert!(
        !client
            .read_caret_bounded_candidate(&candidate, &metadata, || true)
            .attempted
    );
    assert!(mock.requests().is_empty());
    for location in [NSNotFound as usize, usize::MAX - 1] {
        let invalid = ContextClient::new(Some("原文"), 0, NSRange::new(location, 0), no_marked());
        assert!(
            TextClient::new(&invalid)
                .live_selection_metadata(|| true)
                .unwrap()
                .selected
                .is_none()
        );
        assert!(invalid.requests().is_empty());
    }
}

#[test]
fn nil_wrong_text_and_wrong_utf16_length_never_verify_candidate() {
    let mut tracker = SentenceTracker::default();
    tracker.commit("原文");
    for text in [None, Some("另文"), Some("短")] {
        let mock = ContextClient::new(text, 0, NSRange::new(2, 0), no_marked());
        let client = TextClient::new(&mock);
        let metadata = client.live_selection_metadata(|| true).unwrap();
        let candidate = tracker
            .replacement_candidate("原文", metadata.selected, false)
            .unwrap();
        let read = client.read_caret_bounded_candidate(&candidate, &metadata, || true);
        assert!(
            read.text
                .as_deref()
                .and_then(|text| candidate.verify_exact(text))
                .is_none()
        );
        assert!(mock.writes().is_empty());
    }
}
