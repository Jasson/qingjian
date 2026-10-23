//! 有界队列中的一个动作；仅连续退格合并数量。

use super::action::Action;

#[derive(Clone, Copy)]
pub(super) struct Entry {
    pub action: Action,

    pub count: u16,
}
