//! 仅驻留内存的草稿暂存区，按应用显式取回，不自动跨输入框恢复。

mod saved;

use std::collections::VecDeque;

pub use saved::SavedDraft;

const CAPACITY: usize = 16;

#[derive(Debug, Default)]
pub struct DraftArchive {
    entries: VecDeque<SavedDraft>,
}

impl DraftArchive {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stash(&mut self, application: &str, committed: &str, raw_keys: &str) -> bool {
        if committed.is_empty() && raw_keys.is_empty() {
            return false;
        }
        if self.entries.back().is_some_and(|last| {
            last.application == application
                && last.committed == committed
                && last.raw_keys == raw_keys
        }) {
            return false;
        }
        if self.entries.len() == CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(SavedDraft {
            application: application.to_owned(),
            committed: committed.to_owned(),
            raw_keys: raw_keys.to_owned(),
        });
        true
    }

    pub fn has_for(&self, application: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.application == application)
    }

    pub fn take_for(&mut self, application: &str) -> Option<SavedDraft> {
        let index = self
            .entries
            .iter()
            .rposition(|entry| entry.application == application)?;
        self.entries.remove(index)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests;
