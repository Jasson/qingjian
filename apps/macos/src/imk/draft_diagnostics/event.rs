//! IMK 回调作用域；显式 finish 记录最终返回值，异常退出也只丢弃诊断。

pub(crate) struct Event {
    pub(super) ticket: Option<u64>,
}

impl Event {
    pub fn prepared(&self, branch: &'static str) {
        super::update(self.ticket, |record| record.prepared = branch);
    }

    pub fn panicked(&self) {
        super::update(self.ticket, |record| record.panicked = true);
    }

    pub fn finish(mut self, handled: bool, current: impl FnOnce() -> bool) {
        super::finish(self.ticket.take(), Some(handled), current);
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        // 未抵达最终返回点时不猜测 handled，仍移除栈帧，不能把后续 IPC 算到旧事件。
        super::finish(self.ticket.take(), None, || false);
    }
}
