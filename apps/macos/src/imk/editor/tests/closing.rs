//! 真实关闭入口在状态重入时保留控制权，并由生产轮询完成延后的关闭。

use super::scope::OwnerScope;
use crate::imk::editor::owner::Owner;
use crate::imk::editor::{CLOSE_PENDING, CURRENT_PANEL, KEYBOARD_OWNED, STATE, discard, tick};

#[test]
fn busy_discard_preserves_native_ownership_until_poll_retries() {
    assert!(STATE.with(|state| state.borrow().is_none()));
    let _owner = OwnerScope::enter(Owner::Visible(42));
    let previous_panel = CURRENT_PANEL.replace(123);
    let previous_keyboard = KEYBOARD_OWNED.replace(true);
    let previous_close = CLOSE_PENDING.replace(false);
    let result = std::panic::catch_unwind(|| {
        STATE.with(|state| {
            let _busy = state.borrow();
            discard();
            assert_eq!(Owner::current(), Owner::Visible(42));
            assert_eq!(CURRENT_PANEL.get(), 123);
            assert!(KEYBOARD_OWNED.get());
            assert!(CLOSE_PENDING.get());
        });
        tick();
        assert_eq!(Owner::current(), Owner::Idle);
        assert_eq!(CURRENT_PANEL.get(), 0);
        assert!(!KEYBOARD_OWNED.get());
        assert!(!CLOSE_PENDING.get());
    });
    CURRENT_PANEL.set(previous_panel);
    KEYBOARD_OWNED.set(previous_keyboard);
    CLOSE_PENDING.set(previous_close);
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}
