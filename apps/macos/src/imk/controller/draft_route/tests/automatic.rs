//! 自动草稿首键直通与外层兜底联合回放，使用真实 Core 状态保留表达式和问字路径。

use qingjian_core::{Engine, ModeKeys};
use qingjian_dictionary::Dictionary;
use qingjian_predict::live::{TerminalDraft, automatic_translation_visible};

use crate::imk::controller::draft_route::DraftKeyRoute;

fn engine() -> Engine {
    Engine::new(Dictionary::parse("你\tni\t100\n").unwrap())
}

#[test]
fn nonlanguage_first_events_pass_outer_fallback_without_creating_hidden_input() {
    let mut engine = engine();
    let draft = TerminalDraft::default();
    for text in ["1", "2", "3", ".", "，", " ", "🙂", "1️⃣", "Ⓜ️"] {
        let mut route = DraftKeyRoute::for_key(18, false, false);
        assert!(route.consume_fallback(true, true, false));
        assert!(route.pass_empty_input(
            Some(text),
            true,
            &draft.source(),
            !engine.composition().is_empty(),
            false,
            engine.takes_question_mark()
        ));
        assert_eq!(route, DraftKeyRoute::PassApplication);
        assert!(
            !route.consume_fallback(true, true, false),
            "外层不能再吞{text}"
        );
        assert!(engine.composition().is_empty());
        assert!(draft.source().is_empty());
        assert!(!automatic_translation_visible("", &draft.source(), false));
    }
    // 随后的字母由 Core 正常开始，之前交给应用的数字不能混入组句。
    assert!(!DraftKeyRoute::passes_empty_input(
        Some("n"),
        true,
        &draft.source(),
        false,
        false,
        false
    ));
    engine.push('n');
    engine.push('i');
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "你");
    assert_eq!(engine.composition().text(), "ni");
}

#[test]
fn existing_numeric_or_chinese_draft_keeps_punctuation_and_spaces() {
    for committed in ["1", "a", "中", " "] {
        let mut draft = TerminalDraft::default();
        draft.commit(committed);
        for text in ["2", ",", " ", "🙂"] {
            assert!(!DraftKeyRoute::passes_empty_input(
                Some(text),
                true,
                &draft.source(),
                false,
                false,
                false
            ));
            assert!(DraftKeyRoute::for_key(18, false, true).consume_fallback(true, true, false));
            assert!(draft.commit(text));
        }
        assert_eq!(draft.source(), format!("{committed}2, 🙂"));
    }
}

#[test]
fn expression_question_configuration_and_open_symbol_panel_keep_core_ownership() {
    let mut engine = engine();
    assert!(!DraftKeyRoute::passes_empty_input(
        Some("v"),
        true,
        "",
        false,
        false,
        false
    ));
    for ch in "v1+2".chars() {
        engine.push(ch);
    }
    assert!(engine.expression_mode());
    assert!(!DraftKeyRoute::passes_empty_input(
        Some("3"),
        true,
        "",
        true,
        false,
        false
    ));
    engine.push('3');
    assert_eq!(engine.composition().text(), "v1+23");
    assert!(!engine.query().unwrap().candidates.items.is_empty());
    engine.clear();
    assert!(DraftKeyRoute::passes_empty_input(
        Some("?"),
        true,
        "",
        false,
        false,
        engine.takes_question_mark()
    ));
    engine.set_mode_keys(ModeKeys {
        question_mark: true,
        ..ModeKeys::default()
    });
    assert!(!DraftKeyRoute::passes_empty_input(
        Some("?"),
        true,
        "",
        false,
        false,
        engine.takes_question_mark()
    ));
    engine.push('?');
    assert!(engine.question_mode());
    assert!(engine.bare_question());
    engine.clear();
    assert!(engine.open_symbol_panel());
    assert!(!DraftKeyRoute::passes_empty_input(
        Some("1"),
        true,
        "",
        false,
        engine.symbol_panel().is_some(),
        false
    ));
    assert!(engine.symbol_panel().is_some());
}

#[test]
fn excluded_wechat_keeps_both_preedit_and_never_enters_automatic_draft_gate() {
    let mut config = qingjian_platform::Config::default();
    config.live_translation.enabled = true;
    config
        .live_translation
        .draft_excluded_apps
        .push("com.tencent.xinWeChat".into());
    config.general.preedit = qingjian_platform::PreeditMode::Both;
    let active = crate::imk::draft_policy::allowed(
        &config.live_translation,
        Some("com.tencent.xinWeChat"),
        false,
    );
    assert!(!active);
    for text in ["1", ".", " ", "nihao"] {
        assert!(!DraftKeyRoute::passes_empty_input(
            Some(text),
            active,
            "",
            false,
            false,
            false
        ));
    }
    assert_eq!(config.general.preedit, qingjian_platform::PreeditMode::Both);
}
