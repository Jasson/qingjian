//! 原文与取消的默认/改键路由、动作消费屏障、按住隔离和有界队列。

mod boundaries;

use qingjian_platform::{DraftShortcut, ShortcutConfig};

use super::{active, event};
use crate::imk::backspace_tap::state::State;
use crate::imk::backspace_tap::{Action, Bindings};

fn typed() -> State<u64> {
    let mut state = active();
    state.publish_bindings(Bindings::for_config(&ShortcutConfig::default()));
    state
}

#[test]
fn enter_numpad_enter_and_escape_queue_configured_actions_once_per_physical_press() {
    for (key, action) in [
        (36, Action::InsertOriginal),
        (76, Action::InsertOriginal),
        (53, Action::CancelDraft),
    ] {
        let mut state = typed();
        assert!(state.receive(event(&state, 10, key, false)));
        for _ in 0..10 {
            assert!(state.receive(event(&state, 10, key, true)));
        }
        let batch = state.batch().unwrap();
        assert_eq!(batch.action, action);
        assert_eq!(batch.count, 1);
        state.finish(batch.generation, batch.serial, 1, true);
        assert!(state.batch().is_none());
        assert!(state.receive(event(&state, 11, key, false)));
        assert!(state.receive(event(&state, 10, key, false)));
        assert_eq!(state.batch().unwrap().action, action);
    }
}

#[test]
fn output_or_cancel_barrier_discards_all_old_actions_and_keeps_each_owned_key_isolated() {
    for first in [36, 53] {
        let mut state = typed();
        for key in [first, 76, 51] {
            assert!(state.receive(event(&state, 10, key, false)));
        }
        let batch = state.batch().unwrap();
        state.action_consumed(batch.generation, batch.serial);
        state.dispatching = true;
        state.publish(2, true, false);
        state.dispatching = false;
        assert!(state.batch().is_none());
        for key in [first, 76, 51] {
            for _ in 0..5 {
                assert!(state.receive(event(&state, 10, key, true)));
            }
            assert!(state.batch().is_none());
        }
        // 原消费动作后的未来新稿不能被旧按住周期输出、取消或退格。
        state.publish(3, true, true);
        for key in [first, 76, 51] {
            assert!(state.receive(event(&state, 10, key, true)));
        }
        assert!(state.batch().is_none());
        for key in [first, 76, 51] {
            assert!(state.receive(event(&state, 11, key, false)));
        }
        assert!(state.receive(event(&state, 10, 36, false)));
        assert_eq!(state.batch().unwrap().action, Action::InsertOriginal);
    }
}

#[test]
fn new_empty_enter_and_escape_pass_immediately_after_the_previous_physical_release() {
    let mut state = typed();
    state.publish(2, true, false);
    for key in [36, 76, 53] {
        assert!(!state.wants_imk_action(key, 0, false));
        assert!(!state.receive(event(&state, 10, key, false)));
    }
    state.publish(3, true, true);
    assert!(state.receive(event(&state, 10, 36, false)));
    let batch = state.batch().unwrap();
    state.action_consumed(batch.generation, batch.serial);
    state.publish(4, true, false);
    assert!(state.receive(event(&state, 10, 36, true)));
    assert!(state.receive(event(&state, 11, 36, false)));
    assert!(!state.receive(event(&state, 10, 36, false)));
}

#[test]
fn remapping_or_disabling_invalidates_queued_action_without_releasing_its_old_repeat() {
    let mut state = typed();
    assert!(state.receive(event(&state, 10, 36, false)));
    let old = state.batch().unwrap();
    let config = ShortcutConfig {
        insert_original: Some(DraftShortcut::Combo("command+o".parse().unwrap())),
        cancel_draft: None,
        ..ShortcutConfig::default()
    };
    state.publish_bindings(Bindings::for_config(&config));
    assert!(!state.current(old.generation, old.serial));
    assert!(state.batch().is_none());
    assert!(state.receive(event(&state, 10, 36, true)));
    assert!(state.receive(event(&state, 11, 36, false)));
    assert!(!state.receive(event(&state, 10, 36, false)));
    assert!(!state.receive(event(&state, 10, 53, false)));
    let mut custom = event(&state, 10, 31, false);
    custom.flags = 1 << 20;
    assert!(state.receive(custom));
    assert_eq!(state.batch().unwrap().action, Action::InsertOriginal);
}

#[test]
fn hand_edited_conflicts_disable_both_new_actions_but_leave_backspace_available() {
    let mut state = typed();
    let config = ShortcutConfig {
        insert_original: Some(DraftShortcut::Combo(
            ShortcutConfig::default().insert_live_translation.unwrap(),
        )),
        ..ShortcutConfig::default()
    };
    assert!(config.draft_shortcuts_conflict());
    state.publish_bindings(Bindings::for_config(&config));
    assert!(!state.receive(event(&state, 10, 36, false)));
    assert!(!state.receive(event(&state, 10, 53, false)));
    assert!(state.receive(event(&state, 10, 51, false)));
    assert_eq!(state.batch().unwrap().action, Action::Backspace);
}

#[test]
fn queue_is_bounded_and_original_cancel_never_coalesce_into_repeated_writes() {
    let mut state = typed();
    for index in 0..200 {
        let key = if index % 2 == 0 { 36 } else { 53 };
        assert!(state.receive(event(&state, 10, key, false)));
        assert!(state.receive(event(&state, 11, key, false)));
    }
    assert_eq!(state.queue.len(), 32);
    let first = state.batch().unwrap();
    assert_eq!(first.count, 1);
    state.action_consumed(first.generation, first.serial);
    assert!(state.batch().is_none());
}

#[test]
fn imk_fallback_requires_native_isolation_before_any_action_can_dispatch() {
    let mut state = typed();
    state.installed = false;
    assert!(state.wants_imk_action(36, 0, false));
    assert!(!state.receive(event(&state, 10, 36, false)));
    assert!(state.batch().is_none());
    assert!(state.content);
    state.installed = true;
    assert!(state.receive(event(&state, 10, 36, false)));
    assert_eq!(state.batch().unwrap().action, Action::InsertOriginal);
}
