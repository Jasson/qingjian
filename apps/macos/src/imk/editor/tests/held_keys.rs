//! 确认键与组合键维持同一按住周期，重复按下和失焦不能提前释放确认。

use crate::imk::editor::keys::HeldKeys;

#[test]
fn enter_escape_and_translation_combo_wait_for_their_key_up() {
    for key in [36, 76, 53, 16] {
        let mut held = HeldKeys::default();
        assert!(held.is_empty());
        held.press(key);
        for _ in 0..20 {
            held.press(key);
            assert!(!held.is_empty());
        }
        held.release(key + 1);
        assert!(!held.is_empty());
        held.release(key);
        assert!(held.is_empty());
        held.release(key);
        assert!(held.is_empty());
    }
}

#[test]
fn every_accepted_key_must_be_released_before_focus_can_leave() {
    let mut held = HeldKeys::default();
    held.press(0);
    held.press(16);
    held.press(36);
    held.release(16);
    assert!(!held.is_empty());
    held.release(36);
    assert!(!held.is_empty());
    held.release(0);
    assert!(held.is_empty());
}

#[test]
fn reacquiring_focus_prunes_only_keys_that_were_already_owned() {
    let mut held = HeldKeys::default();
    held.press(16);
    held.press(36);
    let mut observed = Vec::new();
    held.retain_pressed(|key| {
        observed.push(key);
        key == 16
    });
    observed.sort_unstable();
    assert_eq!(observed, [16, 36]);
    assert!(!held.is_empty());
    held.release(16);
    assert!(held.is_empty());
}
