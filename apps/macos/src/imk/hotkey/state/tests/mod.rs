//! Mock registry 回放：生命周期、注销失败重试、注册冲突、旧队列与按住/双入口去重。

use super::State;
use crate::imk::hotkey::binding::Binding;
use qingjian_platform::{KeyCombo, Modifiers};

fn binding(key: char, owner: usize) -> Binding {
    Binding {
        combo: KeyCombo {
            key,
            modifiers: Modifiers {
                command: true,
                shift: true,
                ..Modifiers::default()
            },
        },
        owner,
    }
}

fn register(state: &mut State<u64>, binding: Binding, handle: usize) -> u32 {
    state.change(Some(binding));
    let (generation, requested) = state.registration_request().unwrap();
    assert_eq!(requested, binding);
    state.publish(1);
    state.complete(generation, Ok(handle));
    generation
}

#[test]
fn activations_and_config_changes_retire_before_register_and_reject_old_events() {
    let mut state = State::default();
    assert!(state.registration_request().is_none());
    let old = register(&mut state, binding('y', 1), 10);
    state.receive(old, true);
    state.change(Some(binding('v', 1)));
    assert_eq!(state.retiring(), Some(10));
    assert!(!state.receive(old, true));
    assert!(state.start_dispatch().is_none());
    assert!(state.registration_request().is_none());
    state.retired(10);
    let (new, _) = state.registration_request().unwrap();
    assert_ne!(old, new);
    state.complete(new, Ok(20));
    state.change(None);
    assert_eq!(state.retiring(), Some(20));
    assert!(!state.current(new));
    state.retired(20);
    assert!(state.registration_request().is_none());
}

#[test]
fn failed_unregister_retains_handle_and_blocks_new_binding_until_retry_succeeds() {
    let mut state = State::default();
    let old = register(&mut state, binding('y', 1), 10);
    state.change(None);
    assert!(state.retirement_failed(-50));
    assert!(!state.retirement_failed(-50));
    assert_eq!(state.retiring(), Some(10));
    state.change(Some(binding('v', 2)));
    assert_eq!(state.retiring(), Some(10));
    assert!(!state.current(old));
    assert!(state.registration_request().is_none());
    state.retired(10);
    let (next, _) = state.registration_request().unwrap();
    state.complete(next, Ok(20));
    assert!(state.current(next));
    assert!(state.error().is_none());
}

#[test]
fn failed_exclusive_register_never_claims_ownership_or_continually_retries() {
    let mut state = State::<u64>::default();
    state.change(Some(binding('y', 1)));
    let (generation, _) = state.registration_request().unwrap();
    state.complete(generation, Err(-9878));
    assert_eq!(state.error(), Some(("注册", -9878)));
    assert!(!state.owns(binding('y', 1).combo));
    assert!(!state.receive(generation, true));
    assert!(state.registration_request().is_none());
    state.change(None);
    state.change(Some(binding('y', 1)));
    assert!(state.registration_request().is_some());
}

#[test]
fn queued_press_is_bound_to_exact_input_and_unchanged_render_does_not_drop_it() {
    let mut state = State::default();
    let generation = register(&mut state, binding('y', 1), 10);
    state.receive(generation, true);
    state.publish(1);
    assert_eq!(state.start_dispatch(), Some((generation, 1)));
    assert!(state.start_dispatch().is_none());
    state.finish(generation, true);
    state.publish(2);
    assert!(state.start_dispatch().is_none());
    state.receive(generation, false);
    state.receive(generation, true);
    assert_eq!(state.start_dispatch(), Some((generation, 2)));
    state.finish(generation, false);
    assert!(state.start_dispatch().is_none());
}

#[test]
fn pressed_repeat_and_ime_ownership_only_permit_one_dispatch_until_release() {
    let mut state = State::default();
    let key = binding('y', 1);
    let generation = register(&mut state, key, 10);
    assert!(state.owns(key.combo)); // IME 分支只消费；Carbon 是唯一输出入口。
    for _ in 0..5 {
        state.receive(generation, true);
    }
    assert_eq!(state.start_dispatch(), Some((generation, 1)));
    state.finish(generation, false);
    state.receive(generation, true);
    assert!(state.start_dispatch().is_none());
    state.receive(generation, false);
    state.receive(generation, true);
    assert!(state.start_dispatch().is_some());
}

#[test]
fn reentry_during_native_register_retires_returned_handle_and_cannot_revive_old_owner() {
    let mut state = State::<u64>::default();
    state.change(Some(binding('y', 1)));
    let (generation, _) = state.registration_request().unwrap();
    state.change(None);
    state.complete(generation, Ok(10));
    assert_eq!(state.retiring(), Some(10));
    assert!(!state.current(generation));
    state.retired(10);
    assert!(state.registration_request().is_none());
}

#[test]
fn abc_mapping_covers_all_letters_and_carbon_modifier_bits() {
    for key in 'a'..='z' {
        assert!(binding(key, 1).valid());
    }
    assert_eq!(binding('y', 1).key_code(), Some(16));
    assert_eq!(binding('y', 1).modifiers(), (1 << 8) | (1 << 9));
    let mut all = binding('v', 1);
    all.combo.modifiers.option = true;
    all.combo.modifiers.control = true;
    assert_eq!(all.modifiers(), (1 << 8) | (1 << 9) | (1 << 11) | (1 << 12));
    assert!(!binding('1', 1).valid());
    all.combo.modifiers = Modifiers::default();
    assert!(!all.valid());
}

#[test]
fn monitor_stops_after_cleanup_and_failed_registration_can_retry_on_reactivation() {
    let mut state = State::<u64>::default();
    assert!(!state.needs_monitor());
    state.change(Some(binding('y', 1)));
    assert!(state.needs_monitor());
    let (generation, _) = state.registration_request().unwrap();
    state.complete(generation, Err(-9868));
    assert!(!state.needs_monitor());
    state.change(None);
    let generation = register(&mut state, binding('y', 1), 10);
    assert!(state.needs_monitor());
    state.change(None);
    assert!(state.needs_monitor());
    assert!(!state.current(generation));
    state.retired(10);
    assert!(!state.needs_monitor());
}

mod application;
