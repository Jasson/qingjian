//! 精确替换计划：范围为 UTF-16，来源只允许已确认上屏且与现场一致的当前句。

use super::SentenceTracker;

#[test]
fn committed_sentence_preserves_trailing_space_and_excludes_previous_sentence() {
    let mut tracker = SentenceTracker::new();
    tracker.commit("上一句。 Hello world  ");
    let plan = tracker
        .replacement_plan("Hello world", "文档上一句。 Hello world  ", 23, 0, false)
        .unwrap();
    assert_eq!(plan.original, "Hello world  ");
    assert_eq!(plan.location_utf16, 10);
    assert_eq!(plan.length_utf16, 13);
    assert_eq!(plan.replacement("你好世界"), "你好世界  ");
    assert!(plan.matches_original("Hello world  "));
    assert!(!plan.matches_original("Hello world "));
}

#[test]
fn unicode_and_newline_whitespace_have_exact_utf16_ranges() {
    let mut tracker = SentenceTracker::new();
    tracker.commit("前一句。😀你好。\n");
    assert_eq!(
        &tracker.committed()[tracker.current_committed_span()],
        "😀你好。\n"
    );
    let plan = tracker
        .replacement_plan("😀你好。", "前一句。😀你好。\n", 10, 0, false)
        .unwrap();
    assert_eq!(plan.location_utf16, 4);
    assert_eq!(plan.length_utf16, 6);
    assert_eq!(plan.replacement("Hello."), "Hello.\n");
    assert!(
        tracker
            .replacement_plan("😀你好。", "前一句。😀你好。\n", 4, 0, false)
            .is_none()
    );
}

#[test]
fn mismatch_composition_selection_and_stale_source_cannot_replace() {
    let mut tracker = SentenceTracker::new();
    tracker.commit("你好");
    assert!(
        tracker
            .replacement_plan("你好", "你好", usize::MAX, 0, false)
            .is_none()
    );
    assert!(
        tracker
            .replacement_plan("旧的原文", "你好", 2, 0, false)
            .is_none()
    );
    assert!(
        tracker
            .replacement_plan("你好", "新输入框", 4, 0, false)
            .is_none()
    );
    assert!(
        tracker
            .replacement_plan("你好", "你好", 2, 1, false)
            .is_none()
    );
    assert!(
        tracker
            .replacement_plan("你好", "你好", 2, 0, true)
            .is_none()
    );
    tracker.preview("世界");
    assert!(
        tracker
            .replacement_plan("你好世界", "你好世界", 4, 0, false)
            .is_none()
    );
    tracker.reset();
    assert!(tracker.replacement_plan("", "", 0, 0, false).is_none());
}

#[test]
fn unverified_candidate_needs_exact_text_and_utf16_before_it_becomes_plan() {
    use crate::live::context::ContextRange;
    let mut tracker = SentenceTracker::default();
    tracker.commit("上一句。😀原文  ");
    let selected = Some(ContextRange {
        start: 10,
        length: 0,
    });
    let candidate = tracker
        .replacement_candidate("😀原文", selected, false)
        .unwrap();
    assert_eq!(
        candidate.range(),
        ContextRange {
            start: 4,
            length: 6
        }
    );
    assert!(candidate.verify_exact("😀原文  ").is_some());
    assert!(candidate.verify_exact("😀另文  ").is_none());
    assert!(candidate.verify_exact("😀原文 ").is_none());
    assert!(
        tracker
            .replacement_candidate("旧译文原文", selected, false)
            .is_none()
    );
    assert!(
        tracker
            .replacement_candidate(
                "😀原文",
                Some(ContextRange {
                    start: 10,
                    length: 1
                }),
                false
            )
            .is_none()
    );
    assert!(
        tracker
            .replacement_candidate("😀原文", selected, true)
            .is_none()
    );
    assert!(
        tracker
            .replacement_candidate(
                "😀原文",
                Some(ContextRange {
                    start: 2,
                    length: 0
                }),
                false
            )
            .is_none()
    );
    assert!(
        tracker
            .replacement_candidate("😀原文", None, false)
            .is_none()
    );
    assert!(
        tracker
            .replacement_candidate(
                "😀原文",
                Some(ContextRange {
                    start: usize::MAX,
                    length: 0
                }),
                false
            )
            .is_none()
    );
    tracker.preview("未上屏");
    assert!(
        tracker
            .replacement_candidate("😀原文未上屏", selected, false)
            .is_none()
    );
    let text = "字".repeat(4097);
    let mut long = SentenceTracker::default();
    long.commit(&text);
    assert!(
        long.replacement_candidate(
            &text,
            Some(ContextRange {
                start: 4097,
                length: 0
            }),
            false
        )
        .is_none()
    );
}

#[test]
fn selected_candidate_requires_complete_source_and_preserves_exact_whitespace() {
    use crate::live::{ReplacementCandidate, context::ContextRange};
    let selected = Some(ContextRange {
        start: 10,
        length: 5,
    });
    let candidate = ReplacementCandidate::for_selection("你说的都对", selected, false).unwrap();
    assert_eq!(
        candidate.range(),
        ContextRange {
            start: 10,
            length: 5
        }
    );
    assert!(candidate.verify_exact("你说的都对").is_some());
    for changed in ["你说得都对", " 你说的都对", "你说的都对 ", "你说"] {
        assert!(candidate.verify_exact(changed).is_none());
    }
    assert!(
        ReplacementCandidate::for_selection(
            "你说的都对",
            Some(ContextRange {
                start: 10,
                length: 0
            }),
            false
        )
        .is_none()
    );
    assert!(
        ReplacementCandidate::for_selection(
            "你说的都对",
            Some(ContextRange {
                start: 10,
                length: 2
            }),
            false
        )
        .is_none()
    );
    assert!(ReplacementCandidate::for_selection("你说的都对", selected, true).is_none());
    assert!(ReplacementCandidate::for_selection("你说的都对", None, false).is_none());
    assert!(
        ReplacementCandidate::for_selection(
            "😀",
            Some(ContextRange {
                start: usize::MAX - 1,
                length: 2
            }),
            false
        )
        .is_none()
    );
}
