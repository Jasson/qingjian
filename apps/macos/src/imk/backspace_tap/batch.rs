//! 单次主线程派发的原会话凭据与退格数量。

use super::action::Action;

pub(crate) struct Batch<T> {
    pub generation: u64,

    pub serial: u64,

    pub owner: usize,

    pub pid: i32,

    pub token: T,

    pub count: u16,

    pub action: Action,
}
