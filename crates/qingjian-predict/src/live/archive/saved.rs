//! 已暂存的应用草稿，只含原文和未完成键序，不保存预译或候选。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedDraft {
    pub committed: String,

    pub raw_keys: String,

    pub application: String,
}
