//! 草稿按键的会话镜像、保序有界队列与每键按住隔离；不调用 native API。

use std::collections::VecDeque;

use super::action::Action;
use super::batch::Batch;
use super::bindings::Bindings;
use super::entry::Entry;
use super::event::Event;
use super::held::HeldKeys;

pub(super) struct State<T> {
    pub owner: Option<usize>,

    pub pid: Option<i32>,

    pub generation: u64,

    pub token: Option<T>,

    pub configured: bool,

    pub content: bool,

    pub installed: bool,

    pub failed: Option<&'static str>,

    pub dispatching: bool,

    bindings: Bindings,

    held: HeldKeys,

    queue: VecDeque<Entry>,

    serial: u64,

    consumed: [Option<(u64, i64)>; 32],

    consumed_index: usize,
}

impl<T> Default for State<T> {
    fn default() -> Self {
        Self {
            owner: None,
            pid: None,
            generation: 1,
            token: None,
            configured: false,
            content: false,
            installed: false,
            failed: None,
            dispatching: false,
            bindings: Bindings::default(),
            held: HeldKeys::default(),
            queue: VecDeque::new(),
            serial: 0,
            consumed: [None; 32],
            consumed_index: 0,
        }
    }
}

impl<T: Eq + Clone> State<T> {
    pub fn bind(&mut self, owner: Option<usize>, pid: Option<i32>) {
        if self.owner != owner || self.pid != pid {
            self.revoke();
            self.owner = owner;
            self.pid = pid;
        }
    }

    pub fn revoke(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.invalidate();
        self.held.clear();
        self.dispatching = false;
        self.installed = false;
        self.configured = false;
        self.content = false;
        self.token = None;
    }

    fn invalidate(&mut self) {
        self.queue.clear();
        self.serial = self.serial.wrapping_add(1);
        self.held.quarantine_all();
    }

    pub fn discard_pending(&mut self) {
        self.invalidate();
    }

    pub fn action_consumed(&mut self, generation: u64, serial: u64) {
        if self.current(generation, serial) {
            self.invalidate();
        }
    }

    pub fn wants_imk_action(&self, key: i64, flags: u64, repeat: bool) -> bool {
        self.configured
            && ((self.content
                && matches!(
                    self.bindings.action(key, flags),
                    Some(
                        Action::InsertOriginal
                            | Action::CancelDraft
                            | Action::PasteDraft
                            | Action::PasteConflict
                    )
                ))
                || self.owns_action_repeat(key, repeat))
    }

    fn owns_action_repeat(&self, key: i64, repeat: bool) -> bool {
        repeat && key != 51 && self.held.owns(key)
    }

    pub fn release_imk(&mut self, key: i64) -> bool {
        self.held.release(key)
    }

    pub fn imk_ticket(&self) -> Option<(u64, u64, T)> {
        Some((self.generation, self.serial, self.token.clone()?))
    }

    pub fn input_matches(&self, generation: u64, serial: u64, token: &T) -> bool {
        self.generation == generation && self.serial == serial && self.token.as_ref() == Some(token)
    }

    pub fn publish_bindings(&mut self, bindings: Bindings) {
        if self.bindings != bindings {
            self.invalidate();
            self.bindings = bindings;
        }
    }

    pub fn publish(&mut self, token: T, configured: bool, content: bool) {
        if !self.dispatching
            && (self.token.as_ref() != Some(&token) || self.configured != configured)
        {
            self.invalidate();
        }
        self.token = Some(token);
        self.configured = configured;
        self.content = content;
        if !content {
            self.held.quarantine_all();
        }
        if !configured {
            self.queue.clear();
            self.held.clear();
        }
    }

    pub fn needs_tap(&self) -> bool {
        self.owner.is_some() && self.pid.is_some() && self.wants_monitor()
    }

    pub fn wants_monitor(&self) -> bool {
        self.configured
            && self.failed.is_none()
            && (self.content || self.held.any() || !self.queue.is_empty())
    }

    pub fn fail(&mut self, notice: &'static str) {
        self.revoke();
        self.failed = Some(notice);
    }

    pub fn receive(&mut self, event: Event) -> bool {
        if event.generation != self.generation || !self.installed {
            return false;
        }
        if matches!(event.kind, 0xffff_fffe | 0xffff_ffff) {
            self.fail("草稿按键接管已被系统停用，请重新激活青简后重试");
            return false;
        }
        if event.secure || (event.pid != 0 && self.pid.map(i64::from) != Some(event.pid)) {
            self.revoke();
            return false;
        }
        if event.kind == 11 {
            return self.held.release(event.key);
        }
        if event.kind != 10 || !self.configured || self.failed.is_some() {
            return false;
        }
        if self.owns_action_repeat(event.key, event.repeat) {
            self.remember(event.timestamp, event.key);
            return true;
        }
        let action = self.bindings.action(event.key, event.flags);
        let Some(action) = action else {
            let old_repeat = event.repeat
                && self.held.owns(event.key)
                && (event.key != 51 || self.held.matches_flags(event.key, event.flags));
            self.invalidate();
            if old_repeat {
                self.remember(event.timestamp, event.key);
                return true;
            }
            return false;
        };
        let owned = self.held.owns(event.key);
        if !(self.content || owned && event.repeat) {
            return false;
        }
        if !owned {
            self.held.acquire(event.key, event.flags);
        }
        if self.content
            && !self.held.quarantined(event.key)
            && (action == Action::Backspace || !event.repeat)
        {
            self.enqueue(action);
        }
        self.remember(event.timestamp, event.key);
        true
    }

    fn remember(&mut self, timestamp: u64, key: i64) {
        self.consumed[self.consumed_index % self.consumed.len()] = Some((timestamp, key));
        self.consumed_index = self.consumed_index.wrapping_add(1);
    }

    fn enqueue(&mut self, action: Action) {
        if action == Action::Backspace
            && let Some(last) = self.queue.back_mut()
            && last.action == action
        {
            last.count = last.count.saturating_add(1);
            return;
        }
        // 同一会话只保留有限的确认意图；超额仍消费按键，不能落入应用触发发送或执行。
        if self.queue.len() < 32 {
            self.queue.push_back(Entry { action, count: 1 });
        }
    }

    pub fn owns_imk_key(&self, timestamp_ns: u64, key: i64) -> bool {
        timestamp_ns > 0
            && self
                .consumed
                .iter()
                .flatten()
                .any(|(timestamp, captured_key)| {
                    *timestamp > 0
                        && *captured_key == key
                        && timestamp.abs_diff(timestamp_ns) <= 1_000
                })
    }

    #[cfg(test)]
    pub fn owns_imk(&self, timestamp_ns: u64) -> bool {
        self.owns_imk_key(timestamp_ns, 51)
    }

    pub fn batch(&mut self) -> Option<Batch<T>> {
        if !self.installed || !self.configured || self.dispatching {
            return None;
        }
        let entry = self.queue.front()?;
        Some(Batch {
            generation: self.generation,
            serial: self.serial,
            owner: self.owner?,
            pid: self.pid?,
            token: self.token.clone()?,
            count: entry.count,
            action: entry.action,
        })
    }

    pub fn finish(&mut self, generation: u64, serial: u64, count: u16, applied: bool) {
        if applied
            && self.current(generation, serial)
            && let Some(front) = self.queue.front_mut()
        {
            front.count = front.count.saturating_sub(count);
            if front.count == 0 {
                self.queue.pop_front();
            }
        }
    }

    pub fn current(&self, generation: u64, serial: u64) -> bool {
        self.installed && self.generation == generation && self.serial == serial
    }
}

#[cfg(test)]
mod tests;
