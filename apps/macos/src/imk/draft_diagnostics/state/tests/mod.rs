//! 微信限定、激活/进程双预算和重入栈归属的生产状态机回放。

use super::{PER_ACTIVATION, PER_PROCESS, State};
use crate::imk::draft_diagnostics::kind::WriteKind;
use crate::imk::draft_diagnostics::snapshot::Snapshot;

fn snapshot(wechat: bool, staged: bool) -> Option<Snapshot> {
    Some(Snapshot {
        wechat,
        staged,
        epoch: 7,
        composing: false,
        draft_present: false,
    })
}

#[test]
fn only_armed_wechat_or_busy_previously_armed_session_is_recorded() {
    let mut state = State::default();
    let ticket = state.begin("down", snapshot(true, true), true, true, false);
    assert!(state.finish(ticket).is_none());
    state.arm(1, true);
    for (before, secure) in [(snapshot(false, true), false), (snapshot(true, true), true)] {
        let ticket = state.begin("down", before, true, true, secure);
        assert!(state.finish(ticket).is_none());
    }
    let ticket = state.begin("down", snapshot(true, false), true, true, false);
    assert!(!state.finish(ticket).unwrap().before.unwrap().staged);
    let ticket = state.begin("down", None, false, false, false);
    let record = state.finish(ticket).unwrap();
    assert_eq!(record.kind, "down");
    assert!(record.before.is_none());
    assert!(!record.current_before);
    assert!(!record.client_present);
    state.disarm(2);
    let ticket = state.begin("up", snapshot(true, true), true, true, false);
    assert!(state.finish(ticket).is_some());
    state.disarm(1);
    let ticket = state.begin("down", None, true, true, false);
    assert!(state.finish(ticket).is_none());
}

#[test]
fn repeated_activation_cannot_reset_process_budget_and_there_is_no_clock_deadline() {
    let mut state = State::default();
    let mut ids = Vec::new();
    for _ in 0..PER_PROCESS / PER_ACTIVATION {
        state.arm(1, true);
        for _ in 0..PER_ACTIVATION {
            let ticket = state.begin("down", snapshot(true, true), true, true, false);
            ids.push(state.finish(ticket).unwrap().id);
        }
        let ticket = state.begin("down", snapshot(true, true), true, true, false);
        assert!(state.finish(ticket).is_none());
    }
    state.arm(1, true);
    let ticket = state.begin("down", snapshot(true, true), true, true, false);
    assert!(state.finish(ticket).is_none());
    assert_eq!(ids, (1..=PER_PROCESS as u64).collect::<Vec<_>>());
}

#[test]
fn nested_callbacks_count_only_their_own_writes_and_unsampled_frames_mask_outer_event() {
    let mut state = State::default();
    state.arm(1, true);
    let outer = state.begin("down", snapshot(true, true), true, true, false);
    state.write(WriteKind::Marked, "attempt");
    state.write(WriteKind::Marked, "blocked");
    let inner = state.begin("up", snapshot(true, true), true, true, false);
    state.record_mut(inner).unwrap().prepared = "pass";
    state.write(WriteKind::Confirmed, "attempt");
    state.write(WriteKind::Confirmed, "sent");
    let suppressed = state.begin("down", snapshot(false, true), true, true, false);
    state.write(WriteKind::Ordinary, "attempt");
    state.write(WriteKind::Ordinary, "sent");
    assert!(state.finish(suppressed).is_none());
    let record = state.finish(inner).unwrap();
    assert_eq!(record.prepared, "pass");
    assert_eq!(record.writes[2].attempted, 1);
    assert_eq!(record.writes[2].sent, 1);
    assert_eq!(record.writes[0].attempted, 0);
    assert_eq!(record.writes[1].sent, 0);
    state.write(WriteKind::Ordinary, "attempt");
    state.write(WriteKind::Ordinary, "blocked");
    let record = state.finish(outer).unwrap();
    assert_eq!(record.writes[0].attempted, 1);
    assert_eq!(record.writes[0].blocked, 1);
    assert_eq!(record.writes[1].attempted, 1);
    assert_eq!(record.writes[1].blocked, 1);
    assert_eq!(record.writes[2].sent, 0);
    assert!(state.finish(inner).is_none());
}

#[test]
fn activation_and_current_owner_deactivation_invalidate_inflight_records_without_breaking_stack() {
    let mut state = State::default();
    state.arm(1, true);
    let old = state.begin("down", snapshot(true, true), true, true, false);
    state.arm(2, true);
    state.write(WriteKind::Marked, "attempt");
    assert!(state.record_mut(old).is_none());
    let new = state.begin("down", snapshot(true, true), true, true, false);
    state.disarm(1);
    state.write(WriteKind::Confirmed, "attempt");
    assert_eq!(state.record_mut(new).unwrap().writes[2].attempted, 1);
    state.disarm(2);
    state.write(WriteKind::Confirmed, "sent");
    assert!(state.record_mut(new).is_none());
    assert!(state.finish(new).is_none());
    assert!(state.finish(old).is_none());
    state.arm(3, true);
    let ticket = state.begin("down", snapshot(true, true), true, true, false);
    let record = state.finish(ticket).unwrap();
    assert_eq!(record.writes[2].sent, 0);
}

mod callback;
