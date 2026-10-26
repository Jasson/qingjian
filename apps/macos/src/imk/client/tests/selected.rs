//! 选区替换只核验用户明确选中的整段，不退回光标前推算。

use super::{context::ContextClient, no_marked};
use crate::imk::TextClient;
use objc2::DeclaredClass;
use objc2_foundation::NSRange;
use qingjian_predict::live::ReplacementCandidate;

#[test]
fn nonempty_exact_selection_is_required_without_trim_or_substring_matching() {
    let mock = ContextClient::new(
        Some("前句。你说的都对 "),
        0,
        NSRange::new(3, 5),
        no_marked(),
    );
    let client = TextClient::new(&mock);
    let metadata = client.live_selection_metadata(|| true).unwrap();
    let candidate =
        ReplacementCandidate::for_selection("你说的都对", metadata.selected, false).unwrap();
    let first = client.read_selected_candidate(&candidate, &metadata, || true);
    let second = client.read_selected_candidate(&candidate, &metadata, || true);
    assert!(
        candidate
            .verify_exact(first.text.as_deref().unwrap())
            .is_some()
    );
    assert!(
        candidate
            .verify_exact(second.text.as_deref().unwrap())
            .is_some()
    );
    mock.ivars().selected.set(NSRange::new(8, 0));
    let empty = client.live_selection_metadata(|| true).unwrap();
    assert!(ReplacementCandidate::for_selection("你说的都对", empty.selected, false).is_none());
    assert!(
        !client
            .read_selected_candidate(&candidate, &empty, || true)
            .attempted
    );
    for range in [NSRange::new(3, 1), NSRange::new(3, 6)] {
        mock.ivars().selected.set(range);
        let selected = client.live_selection_metadata(|| true).unwrap();
        assert!(
            ReplacementCandidate::for_selection("你说的都对", selected.selected, false).is_none()
        );
    }
    assert!(candidate.verify_exact("你说得都对").is_none());
    assert!(mock.writes().is_empty());
}

#[test]
fn changed_range_marked_text_or_other_client_cannot_reuse_verified_selection_candidate() {
    let mock = ContextClient::new(
        Some("前句。你说的都对下一句"),
        0,
        NSRange::new(3, 5),
        no_marked(),
    );
    let client = TextClient::new(&mock);
    let original = client.live_selection_metadata(|| true).unwrap();
    let candidate =
        ReplacementCandidate::for_selection("你说的都对", original.selected, false).unwrap();
    let first = client.read_selected_candidate(&candidate, &original, || true);
    assert!(
        candidate
            .verify_exact(first.text.as_deref().unwrap())
            .is_some()
    );
    mock.ivars().selected.set(NSRange::new(4, 5));
    let moved = client.live_selection_metadata(|| true).unwrap();
    let reads = mock.requests().len();
    assert!(
        !client
            .read_selected_candidate(&candidate, &moved, || true)
            .attempted
    );
    assert_eq!(mock.requests().len(), reads);
    mock.ivars().selected.set(NSRange::new(3, 5));
    mock.ivars().marked.set(NSRange::new(3, 5));
    let composing = client.live_selection_metadata(|| true).unwrap();
    assert!(
        !client
            .read_selected_candidate(&candidate, &composing, || true)
            .attempted
    );
    let another = ContextClient::new(
        Some("前句。你说的都对下一句"),
        0,
        NSRange::new(3, 5),
        no_marked(),
    );
    assert!(
        !TextClient::new(&another)
            .read_selected_candidate(&candidate, &original, || true)
            .attempted
    );
    assert!(mock.writes().is_empty());
    assert!(another.writes().is_empty());
}
