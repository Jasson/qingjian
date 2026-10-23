//! 草稿整段编辑、精确译文确认与终端单行输出约束。

use super::{TerminalDraft, normalize_terminal_output};

#[test]
fn full_draft_keeps_punctuation_and_preview_commit_never_duplicates() {
    let mut draft = TerminalDraft::default();
    assert!(draft.commit("你好。下一句，"));
    draft.preview("当前高亮");
    assert_eq!(draft.source(), "你好。下一句，当前高亮");
    assert!(
        draft
            .translation_output(&draft.source(), "translation", true)
            .is_none()
    );
    let source = draft.source();
    let revision = draft.revision();
    draft.commit("当前高亮");
    assert!(draft.revision() != revision);
    assert_eq!(draft.source(), source);
    // 高亮预译已成功且源文未变：选字后即可确认，不需要额外请求。
    assert_eq!(
        draft
            .translation_output(&source, "Hello. Next sentence.", true)
            .as_deref(),
        Some("Hello. Next sentence.")
    );
    draft.backspace();
    assert_eq!(draft.source(), "你好。下一句，当前高");
    assert!(draft.translation_output(&source, "old", true).is_none());
}

#[test]
fn pending_mismatch_unselected_and_empty_cannot_output() {
    let mut draft = TerminalDraft::default();
    assert!(draft.original_output().is_none());
    draft.commit("原文");
    assert!(draft.translation_output("原文", "cached", false).is_none());
    assert!(draft.translation_output("旧原文", "cached", true).is_none());
    draft.preview("还没选字");
    assert!(draft.original_output().is_none());
    draft.preview("");
    assert_eq!(draft.original_output().as_deref(), Some("原文"));
    draft.reset();
    assert!(draft.translation_output("原文", "cached", true).is_none());
    draft.commit("新段落");
    assert_eq!(draft.source(), "新段落");
}

#[test]
fn terminal_output_normalizes_line_breaks_and_rejects_all_control_sequences() {
    assert_eq!(
        normalize_terminal_output("one\r\ntwo\nthree\rfour\u{2028}five\u{2029}six").as_deref(),
        Some("one two three four five six")
    );
    for control in (0u8..=31).chain([127, 133]) {
        if matches!(control, b'\r' | b'\n') {
            continue;
        }
        assert!(
            normalize_terminal_output(&format!("before{}after", char::from(control))).is_none()
        );
    }
    let mut draft = TerminalDraft::default();
    draft.commit("原文");
    assert!(!draft.commit("\u{1b}[31m执行"));
    assert_eq!(draft.source(), "原文");
    assert!(
        draft
            .translation_output("原文", "\u{1b}[31m", true)
            .is_none()
    );
    assert_eq!(
        draft
            .translation_output("原文", "line1\r\nline2", true)
            .as_deref(),
        Some("line1 line2")
    );
    draft.commit("\n下一行");
    assert_eq!(draft.original_output().as_deref(), Some("原文 下一行"));
}

#[test]
fn tail_editing_and_reset_do_not_import_the_previous_draft() {
    let mut draft = TerminalDraft::default();
    draft.commit("English ");
    draft.commit("中文");
    draft.commit("！");
    draft.backspace();
    assert_eq!(draft.source(), "English 中文");
    draft.preview("未确认");
    draft.reset();
    draft.preview("新候选");
    draft.commit("新候选");
    assert_eq!(draft.source(), "新候选");
}

#[test]
fn raw_original_combines_confirmed_and_typed_keys_without_the_highlighted_preview() {
    let mut draft = TerminalDraft::default();
    draft.commit("今天");
    draft.preview("是");
    assert_eq!(
        draft.original_output_with_pending("shi").as_deref(),
        Some("今天shi")
    );
    assert_eq!(draft.source(), "今天是");
    assert!(
        draft
            .translation_output("今天是", "Today is", true)
            .is_none()
    );
    assert_eq!(
        draft.original_output_with_pending("ud").as_deref(),
        Some("今天ud")
    );
    assert!(draft.original_output_with_pending("\u{1b}[31m").is_none());
    draft.reset();
    assert_eq!(
        draft.original_output_with_pending("ni").as_deref(),
        Some("ni")
    );
    assert!(draft.original_output_with_pending(" ").is_none());
}
