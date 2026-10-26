//! 开窗准备阶段的控制权回滚，异常或提前返回不会留下永久阻断旧输入的状态。

use super::owner::Owner;

pub(super) struct OpeningGuard {
    ticket: u64,
}

impl OpeningGuard {
    pub(super) fn new() -> Self {
        Self {
            ticket: Owner::generation(),
        }
    }
}

impl Drop for OpeningGuard {
    fn drop(&mut self) {
        if Owner::generation() != self.ticket || !matches!(Owner::current(), Owner::Opening(_)) {
            return;
        }
        super::STATE.with(|state| {
            if let Ok(state) = state.try_borrow()
                && let Some(runtime) = state.as_ref()
            {
                runtime.window.hide();
            }
        });
        super::KEYBOARD_OWNED.set(false);
        Owner::Idle.set("opening_rollback");
    }
}
