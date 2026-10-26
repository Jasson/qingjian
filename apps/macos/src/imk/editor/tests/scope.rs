//! 每个回放临时修改线程内控制权，断言失败展开时也恢复原状态。

use crate::imk::editor::owner::Owner;

pub(super) struct OwnerScope(Owner);

impl OwnerScope {
    pub fn enter(owner: Owner) -> Self {
        let previous = Owner::current();
        owner.set("test_enter");
        Self(previous)
    }
}

impl Drop for OwnerScope {
    fn drop(&mut self) {
        self.0.set("test_restore");
    }
}
