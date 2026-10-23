//! 确认动作的修饰键变化、IMK 重入凭据与精确逐键去重。

use qingjian_platform::{DraftShortcut, ShortcutConfig};

use super::typed;
use crate::imk::backspace_tap::Bindings;
use crate::imk::backspace_tap::state::tests::event;

#[test]
fn modifier_changes_cannot_release_owned_enter_escape_or_letter_repeat() {
    for key in [36, 76, 53, 31] {
        let mut state = typed();
        let mut pressed = event(&state, 10, key, false);
        if key == 31 {
            let config = ShortcutConfig {
                insert_original: Some(DraftShortcut::Combo("command+o".parse().unwrap())),
                ..ShortcutConfig::default()
            };
            state.publish_bindings(Bindings::for_config(&config));
            pressed.flags = 1 << 20;
        }
        assert!(state.receive(pressed));
        let queued = state.batch().unwrap();
        for flags in [0, 1 << 17, 1 << 20, 1 << 23] {
            let mut repeated = event(&state, 10, key, true);
            repeated.flags = flags;
            assert!(state.wants_imk_action(key, flags, true));
            assert!(state.receive(repeated));
            // dispatch 前的修饰键变化也不能取消真正的首个确认动作。
            assert!(state.current(queued.generation, queued.serial));
            assert_eq!(state.batch().unwrap().count, 1);
        }
        state.action_consumed(queued.generation, queued.serial);
        state.publish(2, true, false);
        for flags in [0, 1 << 17, 1 << 20] {
            let mut repeated = event(&state, 10, key, true);
            repeated.flags = flags;
            assert!(state.receive(repeated));
            assert!(state.batch().is_none());
        }
        assert!(state.receive(event(&state, 11, key, false)));
        assert!(!state.receive(event(&state, 10, key, false)));
    }
}

#[test]
fn imk_queue_ticket_cannot_be_rebound_after_native_creation_reentry() {
    for changed in 0..3 {
        let mut state = typed();
        let (generation, serial, token) = state.imk_ticket().unwrap();
        assert!(state.input_matches(generation, serial, &token));
        match changed {
            0 => state.publish(2, true, true),
            1 => state.publish_bindings(Bindings::default()),
            _ => state.bind(Some(11), Some(20)),
        }
        assert!(!state.input_matches(generation, serial, &token));
    }
}

#[test]
fn timestamp_ownership_is_specific_to_the_captured_physical_action_key() {
    let mut state = typed();
    assert!(state.receive(event(&state, 10, 36, false)));
    assert!(state.owns_imk_key(100_000, 36));
    assert!(!state.owns_imk_key(100_000, 53));
    assert!(!state.owns_imk_key(100_000, 51));
    assert!(!state.owns_imk_key(100_000, 0));
    assert!(!state.owns_imk_key(200_000, 36));
}
