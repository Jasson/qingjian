//! 生产 RAII guard 的异常退出清理；诊断失败不泄漏事件归属。

use super::{STATE, event::Event, isolated};

#[test]
fn unwinding_event_guard_is_removed_before_later_callback_and_isolation_preserves_result() {
    let ticket = STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.arm(1, true);
        state.begin("down", None, true, false, false)
    });
    assert!(
        isolated(|| {
            let _guard = Event {
                ticket: Some(ticket),
            };
            panic!("diagnostic guard unwind fixture");
        })
        .is_none()
    );
    STATE.with(|state| assert!(state.borrow_mut().finish(ticket).is_none()));
    let ticket = STATE.with(|state| state.borrow_mut().begin("up", None, true, false, false));
    Event {
        ticket: Some(ticket),
    }
    .finish(false, || true);
    STATE.with(|state| assert!(state.borrow_mut().finish(ticket).is_none()));
    assert_eq!(isolated(|| false), Some(false));
    assert_eq!(isolated(|| true), Some(true));
}

pub(crate) fn capture_writes(action: impl FnOnce()) -> [(u16, u16, u16); 4] {
    let ticket = STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.arm(1, true);
        state.begin("down", None, true, true, false)
    });
    let guard = Event {
        ticket: Some(ticket),
    };
    action();
    let writes = STATE.with(|state| state.borrow_mut().finish(ticket).unwrap().writes);
    drop(guard);
    writes.map(|count| (count.attempted, count.blocked, count.sent))
}

mod base;
mod child;
mod forwarding;
mod state;
