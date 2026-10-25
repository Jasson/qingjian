//! 实际 Engine 与 Archive 回放：切换不自动恢复，明确恢复后继续尾部编辑。

use super::{restore, retain, snapshot};
use crate::host::live::editing::backspace;
use qingjian_core::{Engine, ShuangpinScheme};
use qingjian_dictionary::Dictionary;
use qingjian_predict::live::{DraftArchive, TerminalDraft};

#[test]
fn confirmed_and_actual_pending_keys_survive_app_switch_without_automatic_cross_app_restore() {
    for (input, shuangpin) in [("NiHao", false), ("nihc", true)] {
        let mut engine = Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap());
        engine.set_application(Some("app.a".to_owned()));
        engine.set_shift_letter_compose(true);
        if shuangpin {
            engine.set_shuangpin(Some(ShuangpinScheme::Xiaohe));
        }
        engine.set_input(input);
        let _ = engine.query();
        let mut draft = TerminalDraft::default();
        draft.commit("今天");
        draft.preview("不应保存的高亮");
        let saved = snapshot(&engine, &draft).unwrap();
        assert_eq!(saved.application, "app.a");
        assert_eq!(saved.committed, "今天");
        assert_eq!(saved.raw_keys, input);
        let mut archive = DraftArchive::new();
        archive.stash(&saved.application, &saved.committed, &saved.raw_keys);
        engine.clear();
        draft.reset();
        engine.set_application(Some("app.b".to_owned()));
        assert!(restore(&mut archive, &mut engine, &mut draft).is_none());
        assert!(engine.composition().is_empty());
        assert!(draft.source().is_empty());
        assert!(archive.has_for("app.a"));
        engine.set_application(Some("app.a".to_owned()));
        // 回到同一 app 也不自动改 Engine，只在明确调用恢复时取回。
        assert!(engine.composition().is_empty());
        assert!(restore(&mut archive, &mut engine, &mut draft).is_some());
        assert!(!archive.has_for("app.a"));
        assert_eq!(draft.source(), "今天");
        assert_eq!(engine.raw_preedit().text, input);
        assert_eq!(
            draft
                .original_output_with_pending(&engine.raw_preedit().text)
                .unwrap(),
            format!("今天{input}")
        );
        assert_eq!(backspace(&mut engine, &mut draft), (true, true));
        assert_eq!(draft.committed(), "今天");
        assert_eq!(engine.raw_preedit().text, &input[..input.len() - 1]);
    }
}

#[test]
fn active_draft_and_failed_raw_restore_never_consume_the_archived_content() {
    let mut engine = Engine::new(Dictionary::parse("").unwrap());
    engine.set_application(Some("app.a".to_owned()));
    let mut draft = TerminalDraft::default();
    let mut archive = DraftArchive::new();
    archive.stash("app.a", "保存正文", "\u{1b}x");
    assert!(restore(&mut archive, &mut engine, &mut draft).is_none());
    assert!(archive.has_for("app.a"));
    assert!(engine.composition().is_empty());
    assert!(draft.source().is_empty());
    draft.commit("当前新稿");
    assert!(restore(&mut archive, &mut engine, &mut draft).is_none());
    assert_eq!(draft.committed(), "当前新稿");
    assert!(archive.has_for("app.a"));
}

#[test]
fn preview_is_not_archived_but_pure_whitespace_confirmed_content_is_preserved() {
    let mut engine = Engine::new(Dictionary::parse("").unwrap());
    engine.set_application(Some("app.a".to_owned()));
    let mut draft = TerminalDraft::default();
    draft.preview("高亮候选");
    assert!(snapshot(&engine, &draft).is_none());
    draft.commit(" ");
    let saved = snapshot(&engine, &draft).unwrap();
    assert_eq!(saved.committed, " ");
    let mut archive = DraftArchive::new();
    archive.stash("app.a", &saved.committed, &saved.raw_keys);
    draft.reset();
    assert!(restore(&mut archive, &mut engine, &mut draft).is_some());
    assert_eq!(draft.committed(), " ");
}

#[test]
fn rejected_output_retains_raw_keys_or_literal_text_after_scheme_change() {
    for same_policy in [true, false] {
        let mut engine = Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap());
        engine.set_application(Some("app.a".to_owned()));
        engine.set_shift_letter_compose(true);
        engine.set_input("NiHao");
        let mut draft = TerminalDraft::default();
        draft.commit("正文");
        let saved = snapshot(&engine, &draft).unwrap();
        engine.clear();
        draft.reset();
        let mut archive = DraftArchive::new();
        retain(&mut archive, &saved, same_policy);
        assert!(restore(&mut archive, &mut engine, &mut draft).is_some());
        assert_eq!(
            draft
                .original_output_with_pending(&engine.raw_preedit().text)
                .as_deref(),
            Some("正文NiHao")
        );
        if same_policy {
            assert_eq!(draft.committed(), "正文");
            assert_eq!(engine.raw_preedit().text, "NiHao");
        } else {
            assert_eq!(draft.committed(), "正文NiHao");
            assert!(engine.raw_preedit().text.is_empty());
        }
    }
}
