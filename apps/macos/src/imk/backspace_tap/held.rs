//! 每个物理键独立跟踪按住周期；草稿清空后仍隔离旧重复至松开。

pub(crate) struct HeldKeys {
    states: [u8; 128],

    flags: [u64; 128],
}

impl Default for HeldKeys {
    fn default() -> Self {
        Self {
            states: [0; 128],
            flags: [0; 128],
        }
    }
}

impl HeldKeys {
    pub fn clear(&mut self) {
        self.states.fill(0);
    }

    pub fn any(&self) -> bool {
        self.states.iter().any(|state| *state != 0)
    }

    pub fn owns(&self, key: i64) -> bool {
        self.get(key) != 0
    }

    pub fn quarantined(&self, key: i64) -> bool {
        self.get(key) == 2
    }

    pub fn acquire(&mut self, key: i64, flags: u64) {
        if let Ok(index) = usize::try_from(key)
            && let Some(state) = self.states.get_mut(index)
        {
            *state = 1;
            self.flags[index] = flags & ((0xf << 17) | (1 << 23));
        }
    }

    pub fn matches_flags(&self, key: i64, flags: u64) -> bool {
        self.owns(key)
            && usize::try_from(key)
                .ok()
                .and_then(|index| self.flags.get(index))
                .is_some_and(|captured| *captured == flags & ((0xf << 17) | (1 << 23)))
    }

    pub fn release(&mut self, key: i64) -> bool {
        if let Ok(index) = usize::try_from(key)
            && let Some(state) = self.states.get_mut(index)
        {
            return std::mem::take(state) != 0;
        }
        false
    }

    pub fn quarantine_all(&mut self) {
        for state in &mut self.states {
            if *state != 0 {
                *state = 2;
            }
        }
    }

    fn get(&self, key: i64) -> u8 {
        usize::try_from(key)
            .ok()
            .and_then(|index| self.states.get(index))
            .copied()
            .unwrap_or(0)
    }
}
