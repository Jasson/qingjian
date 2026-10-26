//! 自动输入空态资格与实际草稿生命周期回放；手动编辑器不受自动启动限制。

mod sentence;

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;

use super::{AutomaticSource, passes_automatic_empty, starts_automatic_source};
use crate::live::{FocusedDraftModel, TerminalDraft};

#[test]
fn letters_han_and_mixed_commits_start_but_numbers_symbols_and_emoji_do_not() {
    for text in ["a", "中", "é", "Ａ", "Ω", "𠮷", "1a", "1中", "hi🙂"] {
        assert!(starts_automatic_source(text), "应启动：{text}");
    }
    for text in [
        "",
        "123",
        "3.14",
        ",!?",
        "，。！？",
        " ",
        "  ",
        "\u{a0}",
        "🙂",
        "👨‍👩‍👧‍👦",
        "🇨🇳",
        "1️⃣",
        "#️⃣",
        "🅰️",
        "Ⓜ️",
        "ℹ️",
    ] {
        assert!(!starts_automatic_source(text), "不能独立启动：{text}");
    }
}

#[test]
fn empty_nonlanguage_input_passes_but_pending_content_and_core_layers_keep_ownership() {
    for text in ["123", "3.14", "!?", "，。", " ", "  ", "🙂", "1️⃣", "Ⓜ️"] {
        assert!(
            passes_automatic_empty(Some(text), "", false, false, false),
            "应透传：{text}"
        );
        assert!(!passes_automatic_empty(
            Some(text),
            "1",
            false,
            false,
            false
        ));
        assert!(!passes_automatic_empty(
            Some(text),
            " ",
            false,
            false,
            false
        ));
        assert!(!passes_automatic_empty(Some(text), "", true, false, false));
        assert!(!passes_automatic_empty(Some(text), "", false, true, false));
    }
    for text in [
        None,
        Some(""),
        Some("\t"),
        Some("\r"),
        Some("\u{1b}"),
        Some("\u{7f}"),
    ] {
        assert!(!passes_automatic_empty(text, "", false, false, false));
    }
    assert!(passes_automatic_empty(Some("?"), "", false, false, false));
    assert!(!passes_automatic_empty(Some("?"), "", false, false, true));
}

#[test]
fn actual_draft_remains_visible_after_deleting_the_last_letter_and_hides_only_when_empty() {
    let mut draft = TerminalDraft::default();
    let mut automatic = AutomaticSource::default();
    assert!(!automatic.visible("", &draft.source(), false));
    assert!(automatic.observe("1a"));
    assert!(draft.commit("1a"));
    assert!(automatic.visible("", &draft.source(), false));
    draft.backspace();
    assert_eq!(draft.source(), "1");
    assert!(automatic.visible("", &draft.source(), false));
    assert!(!passes_automatic_empty(
        Some("."),
        &draft.source(),
        false,
        false,
        false
    ));
    draft.backspace();
    automatic.finish(&draft.source(), false);
    assert!(!automatic.visible("", &draft.source(), false));
    assert!(automatic.observe(" a"));
    draft.commit(" a");
    draft.backspace();
    automatic.finish(&draft.source(), false);
    assert_eq!(draft.source(), " ");
    assert!(automatic.visible("", &draft.source(), false));
    draft.backspace();
    automatic.finish(&draft.source(), false);
    assert!(!automatic.observe(" "));
    draft.commit(" ");
    assert!(!automatic.visible("", &draft.source(), false));
    automatic.restore();
    assert!(automatic.visible("", &draft.source(), false));
    assert!(automatic.visible("", "", true));
    assert!(automatic.visible("普通实时翻译", "", false));
}

#[test]
fn unstarted_numeric_source_cannot_show_but_explicit_restore_can_show_it() {
    let mut automatic = AutomaticSource::default();
    for text in ["123", "?!", " ", "🙂"] {
        assert!(!automatic.observe(text));
        assert!(!automatic.visible(text, "", false));
        assert!(!automatic.visible("", text, false));
    }
    assert!(!automatic.visible("", "", false));
    automatic.restore();
    assert!(automatic.visible("123", "123", false));
    automatic.finish("123", false);
    assert!(automatic.visible("123", "123", false));
    automatic.finish("", false);
    assert!(!automatic.observe("123"));
    assert!(!automatic.visible("123", "123", false));
}

#[test]
fn composition_keeps_a_started_session_until_the_actual_input_is_empty() {
    let mut automatic = AutomaticSource::default();
    assert!(automatic.observe("n"));
    automatic.finish("", true);
    assert!(automatic.visible("", "", true));
    assert!(automatic.observe("1"));
    assert!(automatic.visible("1", "", false));
    automatic.finish("", false);
    assert!(!automatic.observe("1"));
    assert!(!automatic.visible("1", "", false));
}

#[test]
fn manual_editor_still_accepts_nonlanguage_first_input_without_the_automatic_gate() {
    for text in ["123", "3.14", "?!", " ", "🙂", "1️⃣"] {
        let mut engine = Engine::new(Dictionary::parse("").unwrap());
        let mut editor = FocusedDraftModel::new();
        assert!(passes_automatic_empty(Some(text), "", false, false, false));
        assert!(editor.text(&mut engine, text, true, true));
        assert_eq!(editor.committed(), text);
        assert_eq!(editor.source(), text);
        assert!(engine.composition().is_empty());
    }
}
