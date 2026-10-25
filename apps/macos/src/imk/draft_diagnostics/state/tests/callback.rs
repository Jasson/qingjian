//! 父类回调的独立预算、事件归属与会话失效，不依赖 OS 或时间窗口。

use super::snapshot;
use crate::imk::draft_diagnostics::state::{
    CALLBACKS_PER_ACTIVATION, CALLBACKS_PER_PROCESS, State,
};

#[test]
fn callback_budget_is_independent_bounded_and_never_resets_process_limit() {
    let mut state = State::default();
    let mut ids = Vec::new();
    for _ in 0..CALLBACKS_PER_PROCESS / CALLBACKS_PER_ACTIVATION {
        state.arm(1, true);
        for _ in 0..CALLBACKS_PER_ACTIVATION {
            let record = state
                .callback("updateComposition", snapshot(true, false), false)
                .unwrap();
            ids.push(record.id);
            assert!(record.parent_event_id.is_none());
            assert!(record.parent_session.is_none());
        }
        assert!(state.callback("originalString", None, false).is_none());
    }
    state.arm(1, true);
    assert!(state.callback("originalString", None, false).is_none());
    let event = state.begin("down", snapshot(true, false), true, true, false);
    assert_eq!(state.finish(event).unwrap().id, 1);
    assert_eq!(ids, (1..=CALLBACKS_PER_PROCESS as u64).collect::<Vec<_>>());
}

#[test]
fn callback_parent_is_only_current_stack_top_and_other_apps_secure_or_inactive_never_record() {
    let mut state = State::default();
    assert!(
        state
            .callback("originalString", snapshot(true, false), false)
            .is_none()
    );
    state.arm(1, true);
    assert!(
        state
            .callback("originalString", snapshot(false, false), false)
            .is_none()
    );
    assert!(
        state
            .callback("originalString", snapshot(true, false), true)
            .is_none()
    );
    let outer = state.begin("down", snapshot(true, false), true, true, false);
    let outer_id = state.record_mut(outer).unwrap().id;
    let record = state
        .callback("cancelComposition", snapshot(true, false), false)
        .unwrap();
    assert_eq!(record.parent_event_id, Some(outer_id));
    assert_eq!(record.parent_session, Some(record.session));
    assert!(state.callback_current(record.session));
    let masked = state.begin("down", snapshot(false, false), true, true, false);
    let nested = state.callback("composedString", None, false).unwrap();
    assert!(nested.parent_event_id.is_none());
    assert!(nested.parent_session.is_none());
    assert!(state.finish(masked).is_none());
    assert_eq!(
        state
            .callback("originalString", None, false)
            .unwrap()
            .parent_event_id,
        Some(outer_id)
    );
    state.finish(outer);
    assert!(
        state
            .callback("originalString", None, false)
            .unwrap()
            .parent_event_id
            .is_none()
    );
    state.disarm(1);
    assert!(!state.callback_current(record.session));
    assert!(state.callback("originalString", None, false).is_none());
    state.arm(2, true);
    assert!(!state.callback_current(record.session));
}
