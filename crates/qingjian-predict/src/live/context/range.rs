//! 宿主文本范围，位置与长度均使用 UTF-16 单元。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextRange {
    pub start: usize,

    pub length: usize,
}

impl ContextRange {
    pub fn end(self) -> Option<usize> {
        self.start.checked_add(self.length)
    }

    pub fn within(self, length: usize) -> bool {
        self.end().is_some_and(|end| end <= length)
    }
}
