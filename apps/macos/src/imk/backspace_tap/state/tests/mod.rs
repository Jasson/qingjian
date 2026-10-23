//! 验证实际路由状态机的作用域、队列、长按空态与失效保护。

mod actions;
mod held;

use super::State;
use crate::imk::backspace_tap::event::Event;

fn active() -> State<u64> {
    let mut state = State::default();
    state.bind(Some(10), Some(20));
    state.publish(1, true, true);
    state.installed = true;
    state
}

fn event(state: &State<u64>, kind: u32, key: i64, repeat: bool) -> Event {
    Event {
        generation: state.generation,
        kind,
        key,
        flags: 0,
        pid: 20,
        timestamp: 100_000,
        repeat,
        secure: false,
    }
}

fn down(state: &mut State<u64>) -> bool {
    state.receive(event(state, 10, 51, false))
}

#[test]
fn only_plain_backspace_in_the_bound_terminal_session_is_owned() {
    for flag in [1 << 18, 1 << 19, 1 << 20, 1 << 23] {
        let mut state = active();
        let mut key = event(&state, 10, 51, false);
        key.flags = flag;
        assert!(!state.receive(key));
        assert!(state.batch().is_none());
    }
    for key in [117, 0, 36] {
        let mut state = active();
        assert!(!state.receive(event(&state, 10, key, false)));
        assert!(state.batch().is_none());
    }
    for invalid in 0..6 {
        let mut state = active();
        let mut key = event(&state, 10, 51, false);
        match invalid {
            0 => key.pid = 99,
            1 => key.secure = true,
            2 => key.generation += 1,
            3 => state.installed = false,
            4 => state.publish(1, false, true),
            _ => state.bind(None, None),
        }
        assert!(!state.receive(key));
        assert!(state.batch().is_none());
    }
    let mut state = active();
    let mut key = event(&state, 10, 51, false);
    // ForPid 已约束进程；字段未标注为0并非显式匹配其他进程。
    key.pid = 0;
    assert!(state.receive(key));
}

#[test]
fn short_press_and_repeat_are_batched_without_losing_own_edits_or_new_arrivals() {
    let mut state = active();
    assert!(down(&mut state));
    assert!(state.receive(event(&state, 11, 51, false)));
    assert!(down(&mut state));
    for _ in 0..8 {
        assert!(state.receive(event(&state, 10, 51, true)));
    }
    let batch = state.batch().unwrap();
    assert_eq!(batch.count, 10);
    // Host 忙时没有执行，原批次仍待处理。
    state.finish(batch.generation, batch.serial, batch.count, false);
    assert_eq!(state.batch().unwrap().count, 10);
    state.dispatching = true;
    assert!(state.batch().is_none());
    state.publish(2, true, true);
    assert!(state.current(batch.generation, batch.serial));
    assert!(state.receive(event(&state, 10, 51, true)));
    state.dispatching = false;
    state.finish(batch.generation, batch.serial, batch.count, true);
    assert_eq!(state.batch().unwrap().count, 1);
}

#[test]
fn external_input_focus_configuration_and_owner_changes_invalidate_pending_deletes() {
    for change in 0..5 {
        let mut state = active();
        assert!(down(&mut state));
        let batch = state.batch().unwrap();
        match change {
            0 => state.publish(2, true, true),
            1 => state.publish(1, false, true),
            2 => state.bind(Some(11), Some(20)),
            3 => state.bind(Some(10), Some(21)),
            _ => {
                state.receive(event(&state, 10, 0, false));
            }
        }
        assert!(!state.current(batch.generation, batch.serial));
        assert!(state.batch().is_none());
    }
}

#[test]
fn disabled_tap_is_latched_until_explicit_reactivation_and_keeps_no_old_intent() {
    for kind in [0xffff_fffe, 0xffff_ffff] {
        let mut state = active();
        assert!(down(&mut state));
        assert!(!state.receive(event(&state, kind, 0, false)));
        assert!(state.failed.is_some());
        state.publish(1, true, true);
        assert!(!state.needs_tap());
        assert!(state.batch().is_none());
        assert!(!down(&mut state));
    }
}

#[test]
fn ime_dedup_owns_only_the_exact_consumed_event_timestamp() {
    let mut state = active();
    assert!(!state.owns_imk(100_000));
    assert!(down(&mut state));
    assert!(state.owns_imk(100_000));
    assert!(state.owns_imk(100_500));
    assert!(!state.owns_imk(200_000));
    assert!(!state.owns_imk(0));
}

#[test]
fn panic_disable_clears_dispatch_guard_and_reactivation_can_dispatch_again() {
    let mut state = active();
    assert!(down(&mut state));
    state.dispatching = true;
    state.revoke();
    assert!(!state.dispatching);
    state.bind(Some(11), Some(20));
    state.publish(2, true, true);
    state.installed = true;
    assert!(down(&mut state));
    assert_eq!(state.batch().unwrap().count, 1);
}

#[test]
fn create_failure_is_visible_latched_and_does_not_claim_keyboard_ownership() {
    let mut state = active();
    state.fail("权限或创建失败");
    assert!(state.failed.is_some());
    for token in 1..10 {
        state.publish(token, true, true);
        assert!(!state.needs_tap());
        assert!(!state.wants_monitor());
        assert!(!down(&mut state));
    }
    // 只有显式 activate 路径会清除此 latch，轮询与输入发布不会清除。
    state.failed = None;
    assert!(state.needs_tap());
    state.installed = true;
    assert!(down(&mut state));
}

#[test]
fn idle_empty_snapshot_needs_no_polling_and_new_content_rearms_monitoring() {
    let mut state = active();
    state.publish(2, true, false);
    assert!(!state.wants_monitor());
    assert!(!state.needs_tap());
    state.publish(3, true, true);
    assert!(state.wants_monitor());
    assert!(state.needs_tap());
}
