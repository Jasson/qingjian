//! 系统热键的纯状态机；注册修订与完整输入快照共同约束排队意图。

use super::binding::Binding;

pub(super) struct State<T> {
    generation: u32,

    desired: Option<Binding>,

    handle: Option<usize>,

    retiring: Option<usize>,

    registration_attempted: bool,

    error: Option<i32>,

    retirement_error: Option<i32>,

    held: bool,

    pending: Option<(u32, T)>,

    in_flight: Option<u32>,

    snapshot: Option<T>,
}

impl<T> Default for State<T> {
    fn default() -> Self {
        Self {
            generation: 0,
            desired: None,
            handle: None,
            retiring: None,
            registration_attempted: false,
            error: None,
            retirement_error: None,
            held: false,
            pending: None,
            in_flight: None,
            snapshot: None,
        }
    }
}

impl<T: Clone + PartialEq> State<T> {
    pub fn change(&mut self, desired: Option<Binding>) {
        if desired == self.desired {
            return;
        }
        let next = self.generation.checked_add(1);
        self.generation = next.unwrap_or(self.generation);
        self.desired = next.and(desired);
        if self.handle.is_some() {
            self.retiring = self.handle.take();
        }
        self.registration_attempted = false;
        self.error = next.is_none().then_some(-50);
        self.held = false;
        self.pending = None;
        self.in_flight = None;
        self.snapshot = None;
    }

    pub fn registration_request(&mut self) -> Option<(u32, Binding)> {
        if self.handle.is_some() || self.retiring.is_some() || self.registration_attempted {
            return None;
        }
        let binding = self.desired?;
        self.registration_attempted = true;
        Some((self.generation, binding))
    }

    /// 若 native 注册期间重入并切换会话，保留新句柄用于注销，不能遗失全局占键。
    pub fn complete(&mut self, generation: u32, result: Result<usize, i32>) {
        if generation != self.generation || self.desired.is_none() {
            if let Ok(handle) = result {
                self.retiring = Some(handle);
            }
            return;
        }
        match result {
            Ok(handle) => self.handle = Some(handle),
            Err(status) => self.error = Some(status),
        }
    }

    pub fn retiring(&self) -> Option<usize> {
        self.retiring
    }

    pub fn retired(&mut self, handle: usize) {
        if self.retiring == Some(handle) {
            self.retiring = None;
            self.retirement_error = None;
        }
    }

    pub fn retirement_failed(&mut self, status: i32) -> bool {
        let changed = self.retirement_error != Some(status);
        self.retirement_error = Some(status);
        changed
    }

    pub fn error(&self) -> Option<(&'static str, i32)> {
        self.retirement_error
            .map(|status| ("注销", status))
            .or_else(|| self.error.map(|status| ("注册", status)))
    }

    pub fn receive(&mut self, generation: u32, pressed: bool) -> bool {
        if !self.current(generation) {
            return false;
        }
        if pressed && !self.held {
            self.held = true;
            self.pending = self.snapshot.clone().map(|token| (generation, token));
            return self.pending.is_some();
        } else if !pressed {
            self.held = false;
        }
        false
    }

    pub fn current(&self, generation: u32) -> bool {
        generation == self.generation && self.desired.is_some() && self.handle.is_some()
    }

    pub fn owns(&self, combo: qingjian_platform::KeyCombo) -> bool {
        self.handle.is_some() && self.desired.is_some_and(|binding| binding.combo == combo)
    }

    pub fn binding(&self, generation: u32) -> Option<Binding> {
        self.current(generation).then_some(self.desired).flatten()
    }

    pub fn tracking(&self) -> bool {
        self.handle.is_some() || (self.desired.is_some() && !self.registration_attempted)
    }

    pub fn needs_monitor(&self) -> bool {
        self.tracking() || self.retiring.is_some() || self.pending.is_some()
    }

    pub fn publish(&mut self, token: T) -> bool {
        let mut dropped = false;
        if self
            .pending
            .as_ref()
            .is_some_and(|(_, pending)| *pending != token)
        {
            self.pending = None;
            dropped = true;
        }
        self.snapshot = Some(token);
        dropped
    }

    pub fn start_dispatch(&mut self) -> Option<(u32, T)> {
        if self.in_flight.is_some() {
            return None;
        }
        let pending = self.pending.clone()?;
        self.in_flight = Some(pending.0);
        Some(pending)
    }

    pub fn finish(&mut self, generation: u32, retry: bool) {
        if self.in_flight != Some(generation) {
            return;
        }
        self.in_flight = None;
        if !retry
            && self
                .pending
                .as_ref()
                .is_some_and(|(queued, _)| *queued == generation)
        {
            self.pending = None;
        }
    }
}

#[cfg(test)]
mod tests;
