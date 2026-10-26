//! 译文回执的修订隔离、失败保留和输出消费回放。

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;

use super::{engine, success};
use crate::live::{FocusedDraftModel, Translation};

#[test]
fn stale_same_source_revision_and_failed_requests_cannot_output_but_keep_source_and_copy() {
    let mut engine = engine();
    let mut model = FocusedDraftModel::new();
    model.text(&mut engine, "hello", true, true);
    success(&mut model, 1, "你好");
    assert_eq!(model.translation_output().as_deref(), Some("你好"));
    let old_source = model.source();
    model.backspace(&mut engine);
    model.text(&mut engine, "o", true, true);
    assert_eq!(model.source(), old_source);
    assert!(model.translation_output().is_none());
    assert!(!model.accept(Translation {
        id: 1,
        source: old_source.clone(),
        result: Ok("旧译文".into())
    }));
    model.submitted(2);
    assert!(model.accept(Translation {
        id: 2,
        source: old_source,
        result: Err("离线".into())
    }));
    assert_eq!(model.error(), Some("离线"));
    assert_eq!(model.copy_output(), Some("你好"));
    assert!(model.translation_output().is_none());
    assert_eq!(model.original_output().as_deref(), Some("hello"));
}

#[test]
fn rejection_preserves_full_original_and_success_consumes_only_once() {
    let mut engine = engine();
    let mut model = FocusedDraftModel::new();
    model.text(&mut engine, "English ", true, true);
    model.text(&mut engine, "ni", false, true);
    assert_eq!(model.original_output().as_deref(), Some("English ni"));
    model.finish_output(&mut engine, false);
    assert_eq!(model.original_output().as_deref(), Some("English ni"));
    assert!(model.commit_raw(&mut engine));
    success(&mut model, 3, "translation");
    assert!(model.translation_output().is_some());
    model.finish_output(&mut engine, true);
    assert!(model.original_output().is_none());
    assert!(model.translation_output().is_none());
    assert!(model.copy_output().is_none());
    assert!(model.raw_keys().is_empty());
    assert!(engine.composition().is_empty());
    assert!(!model.accept(Translation {
        id: 3,
        source: "English ni".into(),
        result: Ok("late".into())
    }));
}

#[test]
fn unchanged_preview_keeps_receipt_but_different_candidate_and_returning_to_it_are_stale() {
    let mut engine = engine();
    let mut model = FocusedDraftModel::new();
    model.text(&mut engine, "nihao", false, true);
    model.submitted(1);
    let reply = Translation {
        id: 1,
        source: model.source(),
        result: Ok("hello".into()),
    };
    let revision = model.revision();
    assert!(model.accept(reply.clone()));
    model.refresh(&mut engine);
    model.move_highlight(0);
    assert_eq!(model.revision(), revision);
    assert!(model.accept(reply.clone()));
    model.move_highlight(1);
    assert_ne!(model.source(), reply.source);
    assert!(!model.accept(reply.clone()));
    model.move_highlight(-1);
    assert_eq!(model.source(), reply.source);
    assert!(!model.accept(reply));
    model.choose(&mut engine, 0);
    success(&mut model, 2, "hello");
    model.refresh(&mut engine);
    assert_eq!(model.translation_output().as_deref(), Some("hello"));
}

#[test]
fn raw_edits_without_any_candidate_invalidate_receipt_even_after_deleting_back_to_same_source() {
    let mut engine = Engine::new(Dictionary::parse("").unwrap());
    let mut model = FocusedDraftModel::new();
    model.text(&mut engine, "hello", true, true);
    success(&mut model, 1, "你好");
    let source = model.source();
    let revision = model.revision();
    assert!(model.translation_output().is_some());
    model.text(&mut engine, "q", false, true);
    assert!(model.candidates().is_empty());
    assert_eq!(model.source(), source);
    assert_ne!(model.revision(), revision);
    model.backspace(&mut engine);
    assert_eq!(model.source(), source);
    assert!(model.raw_keys().is_empty());
    assert_ne!(model.revision(), revision);
    assert!(model.translation_output().is_none());
    assert_eq!(model.copy_output(), Some("你好"));
    assert!(!model.accept(Translation {
        id: 1,
        source,
        result: Ok("old".into())
    }));
    success(&mut model, 2, "重新确认");
    assert_eq!(model.translation_output().as_deref(), Some("重新确认"));
}

#[test]
fn replacing_a_translator_and_reusing_its_request_id_requires_a_new_success() {
    let mut engine = engine();
    let mut model = FocusedDraftModel::new();
    model.text(&mut engine, "hello", true, true);
    success(&mut model, 1, "旧成功译文");
    let old_revision = model.revision();
    model.invalidate_translation();
    assert_ne!(model.revision(), old_revision);
    assert!(model.translation_output().is_none());
    assert_eq!(model.copy_output(), Some("旧成功译文"));
    model.submitted(1);
    assert!(model.translation_output().is_none());
    assert!(model.accept(Translation {
        id: 1,
        source: model.source(),
        result: Err("新服务离线".into())
    }));
    assert!(model.translation_output().is_none());
    assert_eq!(model.copy_output(), Some("旧成功译文"));
    model.submitted(1);
    assert!(model.translation_output().is_none());
    assert!(model.accept(Translation {
        id: 1,
        source: model.source(),
        result: Ok("新成功译文".into())
    }));
    assert_eq!(model.translation_output().as_deref(), Some("新成功译文"));
    model.submitted(1);
    assert!(model.translation_output().is_none());
}
