//! 待填入确认只接受同一修订和同一文字，单次领取后需要明确拒绝才能重试。

use qingjian_predict::live::{FocusedDraftModel, Translation};

use super::engine;
use crate::imk::editor::{keys::HeldKeys, pending::Pending};

fn original(model: &FocusedDraftModel) -> Pending {
    Pending {
        id: 41,
        text: model.original_output().unwrap(),
        original: true,
        revision: model.revision(),
        attempts: 0,
        manual: false,
        claimed: false,
    }
}

#[test]
fn edits_and_returning_to_identical_text_invalidate_old_confirmation() {
    let mut engine = engine();
    let mut model = FocusedDraftModel::new();
    model.text(&mut engine, "draft", true, true);
    let mut pending = original(&model);
    model.text(&mut engine, "x", true, true);
    assert!(!pending.matches(&model));
    assert!(pending.claim(&model).is_none());
    model.backspace(&mut engine);
    assert_eq!(model.original_output().as_deref(), Some("draft"));
    assert!(!pending.matches(&model));
    assert!(pending.claim(&model).is_none());
    assert!(!pending.claimed);
}

#[test]
fn a_held_confirm_key_can_only_consume_one_pending_output() {
    let mut engine = engine();
    let mut model = FocusedDraftModel::new();
    model.text(&mut engine, "nihao", false, true);
    let mut pending = original(&model);
    let mut held = HeldKeys::default();
    held.press(36);
    assert_eq!(pending.claim(&model).as_deref(), Some("nihao"));
    for _ in 0..20 {
        held.press(36);
        assert!(pending.claim(&model).is_none());
    }
    assert!(!held.is_empty());
    held.release(36);
    assert!(held.is_empty());
    assert!(pending.claim(&model).is_none());
    pending.rejected();
    assert!(pending.manual);
    assert_eq!(pending.claim(&model).as_deref(), Some("nihao"));
    assert!(pending.claim(&model).is_none());
}

#[test]
fn pending_translation_rejects_changed_receipt_text_and_old_reply_id() {
    let mut engine = engine();
    let mut model = FocusedDraftModel::new();
    model.text(&mut engine, "你好", true, true);
    model.submitted(10);
    assert!(model.accept(Translation {
        id: 10,
        source: "你好".into(),
        result: Ok("Hello".into())
    }));
    let mut pending = original(&model);
    pending.original = false;
    pending.text = "Hello".into();
    assert!(pending.matches(&model));
    model.submitted(11);
    assert!(!model.accept(Translation {
        id: 10,
        source: "你好".into(),
        result: Ok("stale".into())
    }));
    assert!(pending.claim(&model).is_none());
    assert!(model.accept(Translation {
        id: 11,
        source: "你好".into(),
        result: Ok("Hi".into())
    }));
    assert!(pending.claim(&model).is_none());
    assert!(!pending.claimed);
}
