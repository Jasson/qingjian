//! 经光标现场校验的精确替换范围；保留句子末尾的空白。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacementPlan {
    pub location_utf16: usize,

    pub length_utf16: usize,

    /// 实际范围内的原文，包含尾部空白，供上屏前再次读取核对。
    pub original: String,
}

impl ReplacementPlan {
    pub fn matches_original(&self, readback: &str) -> bool {
        self.original == readback
    }

    pub fn replacement(&self, translation: &str) -> String {
        let trailing = &self.original[self.original.trim_end().len()..];
        format!("{translation}{trailing}")
    }
}
