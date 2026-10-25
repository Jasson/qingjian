//! 自有面板的物理按住周期；所有已接收按键松开前不释放确认焦点。

use std::collections::HashSet;

#[derive(Default)]
pub(super) struct HeldKeys {
    keys: HashSet<u16>,
}

impl HeldKeys {
    pub fn press(&mut self, key: u16) {
        self.keys.insert(key);
    }

    pub fn release(&mut self, key: u16) {
        self.keys.remove(&key);
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// 失焦重获时只查询已经属于本面板的键，不观察新键。
    pub fn retain_pressed(&mut self, mut pressed: impl FnMut(u16) -> bool) {
        self.keys.retain(|key| pressed(*key));
    }
}
