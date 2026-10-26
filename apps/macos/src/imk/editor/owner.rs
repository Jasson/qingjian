//! 原生草稿工作流控制权；独立于瞬时键盘焦点，重入时不依赖可变借用。

use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    Idle,
    Opening(i32),
    Visible(i32),
    Pending(i32),
}

thread_local! {
    static OWNER: Cell<Owner> = const { Cell::new(Owner::Idle) };
    static GENERATION: Cell<u64> = const { Cell::new(0) };
}

impl Owner {
    pub(crate) fn generation() -> u64 {
        GENERATION.get()
    }

    pub(crate) fn current() -> Self {
        OWNER.get()
    }

    pub(crate) fn controlled() -> bool {
        Self::current() != Self::Idle
    }

    pub(crate) fn legacy_route<R>(action: impl FnOnce() -> R) -> Option<R> {
        if Self::controlled() {
            None
        } else {
            Some(action())
        }
    }

    pub(crate) fn pid(self) -> Option<i32> {
        match self {
            Self::Idle => None,
            Self::Opening(pid) | Self::Visible(pid) | Self::Pending(pid) => Some(pid),
        }
    }

    pub(crate) fn set(self, reason: &'static str) {
        GENERATION.set(GENERATION.get().wrapping_add(1));
        let previous = OWNER.replace(self);
        if previous != self {
            tracing::info!(target: "live_translation", marker = "focused-owner-v1", ?previous, next = ?self, reason, "草稿会话控制权变化");
        }
    }

    pub(crate) fn departed(self, pid: i32) -> bool {
        self.pid()
            .is_some_and(|owner| pid > 0 && pid != owner && pid != std::process::id() as i32)
    }
}
