//! 同一退格按住周期删空后隔离；新稿与修饰键不能提前解除隔离。

use super::{active, down, event};

#[test]
fn held_repeat_after_empty_never_deletes_application_text_but_next_empty_press_passes() {
    let mut state = active();
    assert!(down(&mut state));
    let batch = state.batch().unwrap();
    state.dispatching = true;
    state.publish(2, true, false);
    state.dispatching = false;
    state.finish(batch.generation, batch.serial, batch.count, true);
    assert!(state.needs_tap());
    for _ in 0..20 {
        assert!(state.receive(event(&state, 10, 51, true)));
    }
    assert!(state.batch().is_none());
    assert!(state.receive(event(&state, 11, 51, false)));
    assert!(!state.needs_tap());
    assert!(!down(&mut state));
}

#[test]
fn external_input_and_new_draft_cannot_reuse_the_old_held_repeat() {
    let mut state = active();
    assert!(down(&mut state));
    assert!(!state.receive(event(&state, 10, 0, false)));
    state.publish(2, true, true);
    for _ in 0..5 {
        assert!(state.receive(event(&state, 10, 51, true)));
    }
    assert!(state.batch().is_none());
    assert!(state.receive(event(&state, 11, 51, false)));
    assert!(down(&mut state));
    assert_eq!(state.batch().unwrap().count, 1);
}

#[test]
fn modified_delete_passes_but_keyup_always_releases_owned_quarantine() {
    for modifier in [1 << 18, 1 << 19, 1 << 20, 1 << 23] {
        let mut state = active();
        assert!(down(&mut state));
        let batch = state.batch().unwrap();
        state.dispatching = true;
        state.publish(2, true, false);
        state.dispatching = false;
        state.finish(batch.generation, batch.serial, batch.count, true);
        let mut modified = event(&state, 10, 51, true);
        modified.flags = modifier;
        assert!(!state.receive(modified));
        assert!(state.receive(event(&state, 10, 51, true)));
        assert!(state.batch().is_none());
        modified.kind = 11;
        assert!(state.receive(modified));
        assert!(!state.needs_tap());
        assert!(!down(&mut state));
    }
}
