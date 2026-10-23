//! 每次激活最多 16 条：开启 1、普通入口心跳 1、空参数 2、删除类别 12。

use super::kind::Kind;
use std::time::Duration;

#[derive(Default)]
pub(super) struct Limiter {
    owner: Option<usize>,

    session: u64,

    total: usize,

    special: usize,

    nil_seen: [bool; 2],

    ordinary_seen: bool,

    ordinary_count: u64,

    last_special: [Option<Duration>; 4],
}

impl Limiter {
    pub fn arm(&mut self, owner: usize) -> u64 {
        let session = self.session.saturating_add(1);
        *self = Self {
            owner: Some(owner),
            session,
            total: 1,
            ..Self::default()
        };
        session
    }

    pub fn stop(&mut self) {
        self.owner = None;
    }

    pub fn session(&self) -> u64 {
        self.session
    }

    pub fn armed_for(&self, owner: usize) -> bool {
        self.owner == Some(owner)
    }

    pub fn disarm(&mut self, owner: usize) {
        if self.owner == Some(owner) {
            self.owner = None;
        }
    }

    pub fn armed(&self) -> bool {
        self.owner.is_some()
    }

    pub fn ordinary_count(&self) -> u64 {
        self.ordinary_count
    }

    pub fn allow(&mut self, kind: Kind, elapsed: Duration, repeat: bool) -> bool {
        if !self.armed() {
            return false;
        }
        if kind == Kind::NonDeleteKeyDown {
            self.ordinary_count = self.ordinary_count.saturating_add(1);
        }
        if self.total >= 16 {
            return false;
        }
        let allowed = if let Some(slot) = kind.special_slot() {
            if self.special >= 12
                || (repeat && self.last_special[slot].is_some())
                || self.last_special[slot]
                    .is_some_and(|last| elapsed.saturating_sub(last) < Duration::from_secs(1))
            {
                false
            } else {
                self.last_special[slot] = Some(elapsed);
                self.special += 1;
                true
            }
        } else {
            match kind {
                Kind::MissingEvent | Kind::MissingClient => {
                    let slot = usize::from(kind == Kind::MissingClient);
                    let allowed = !self.nil_seen[slot];
                    self.nil_seen[slot] = true;
                    allowed
                }
                Kind::NonDeleteKeyDown if !self.ordinary_seen => {
                    self.ordinary_seen = true;
                    true
                }
                _ => false,
            }
        };
        if allowed {
            self.total += 1;
        }
        allowed
    }
}

#[cfg(test)]
mod tests;
