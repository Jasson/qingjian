//! 原始按键、候选与双拼输入复用 Core 的回放。

use qingjian_core::ShuangpinScheme;

use super::{engine, success};
use crate::live::FocusedDraftModel;

#[test]
fn chinese_candidates_preview_choose_and_punctuation_use_core_without_duplicate_commit() {
    let mut engine = engine();
    engine.set_full_width_punctuation(true);
    let mut model = FocusedDraftModel::new();
    assert!(model.text(&mut engine, "nihao", false, true));
    assert_eq!(model.committed(), "");
    assert_eq!(model.source(), "你好");
    assert_eq!(model.raw_keys(), "nihao");
    success(&mut model, 1, "Hello");
    assert!(model.translation_output().is_none());
    model.move_highlight(1);
    assert_eq!(model.source(), "你号");
    model.move_highlight(-1);
    assert_eq!(model.source(), "你好");
    assert!(model.choose(&mut engine, 0));
    assert_eq!(model.committed(), "你好");
    assert_eq!(model.source(), "你好");
    assert!(model.translation_output().is_none());
    model.text(&mut engine, ",", false, true);
    assert_eq!(model.committed(), "你好，");
    assert!(!model.choose(&mut engine, usize::MAX));
    model.reset(&mut engine);
    model.text(&mut engine, "nihao,", false, true);
    assert_eq!(model.committed(), "你好，");
}

#[test]
fn shuangpin_and_shifted_letters_keep_the_core_input_semantics() {
    let mut engine = engine();
    engine.set_shuangpin(Some(ShuangpinScheme::Xiaohe));
    let mut model = FocusedDraftModel::new();
    let keys: String = ["ni", "hao"]
        .into_iter()
        .flat_map(|syllable| ShuangpinScheme::Xiaohe.encode(syllable).unwrap())
        .collect();
    model.text(&mut engine, &keys, false, true);
    assert_eq!(model.source(), "你好");
    model.text(&mut engine, "A", false, true);
    assert_eq!(model.committed(), format!("{keys}A"));
    assert!(model.raw_keys().is_empty());
    model.reset(&mut engine);
    engine.set_shuangpin(None);
    engine.set_shift_letter_compose(true);
    model.text(&mut engine, "Nihao", false, true);
    assert_eq!(model.raw_keys(), "Nihao");
    assert_eq!(model.source(), "你好");
}

#[test]
fn rejected_control_text_does_not_partially_mutate_a_draft() {
    let mut engine = engine();
    let mut model = FocusedDraftModel::new();
    model.text(&mut engine, "keep", true, true);
    assert!(!model.text(&mut engine, "bad\u{1b}", true, true));
    assert_eq!(model.source(), "keep");
    assert!(model.backspace(&mut engine));
    assert_eq!(model.source(), "kee");
}

#[test]
fn backspace_edits_shuangpin_keys_before_committed_english() {
    let mut engine = engine();
    let mut model = FocusedDraftModel::new();
    model.text(&mut engine, "English ", true, true);
    engine.set_shuangpin(Some(ShuangpinScheme::Xiaohe));
    let keys: String = ["ni", "hao"]
        .into_iter()
        .flat_map(|syllable| ShuangpinScheme::Xiaohe.encode(syllable).unwrap())
        .collect();
    model.text(&mut engine, &keys, false, true);
    for remaining in (0..keys.len()).rev() {
        assert!(model.backspace(&mut engine));
        assert_eq!(model.raw_keys(), &keys[..remaining]);
        assert_eq!(model.committed(), "English ");
    }
    assert!(model.backspace(&mut engine));
    assert_eq!(model.committed(), "English");
}
