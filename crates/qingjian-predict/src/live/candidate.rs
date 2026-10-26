//! 仅根据受控原句和现场光标推导的待核验短范围，不能直接授权写入。

use super::ReplacementPlan;
use super::context::ContextRange;

pub const MAX_REPLACEMENT_UNITS: usize = 4096;

#[derive(Clone, PartialEq, Eq)]
pub struct ReplacementCandidate {
    range: ContextRange,

    original: String,
}

impl ReplacementCandidate {
    pub(super) fn new(original: &str, selected: ContextRange) -> Option<Self> {
        let length = original.encode_utf16().count();
        if selected.length != 0
            || selected.start == usize::MAX
            || length == 0
            || length > MAX_REPLACEMENT_UNITS
        {
            return None;
        }
        let range = ContextRange {
            start: selected.start.checked_sub(length)?,
            length,
        };
        (range.end()? == selected.start).then(|| Self {
            range,
            original: original.to_owned(),
        })
    }

    pub fn range(&self) -> ContextRange {
        self.range
    }

    pub fn matches_caret(&self, selected: Option<ContextRange>, has_active_marked: bool) -> bool {
        !has_active_marked
            && selected.is_some_and(|selected| {
                selected.length == 0
                    && selected.start != usize::MAX
                    && self.range.end() == Some(selected.start)
                    && self.range.length > 0
                    && self.range.length <= MAX_REPLACEMENT_UNITS
            })
    }

    pub fn verify_exact(&self, readback: &str) -> Option<ReplacementPlan> {
        (readback.encode_utf16().count() == self.range.length && readback == self.original).then(
            || ReplacementPlan {
                location_utf16: self.range.start,
                length_utf16: self.range.length,
                original: self.original.clone(),
            },
        )
    }
}

impl ReplacementCandidate {
    /// 仅为用户当前非空选区建立候选；选中内容稍后必须完整等于成功请求的原文。
    pub fn for_selection(
        successful_source: &str,
        selected: Option<ContextRange>,
        has_active_marked: bool,
    ) -> Option<Self> {
        let selected = selected?;
        let length = successful_source.encode_utf16().count();
        if has_active_marked
            || selected.length == 0
            || selected.start == usize::MAX
            || length == 0
            || length > MAX_REPLACEMENT_UNITS
            || selected.length != length
            || selected.end().is_none()
        {
            return None;
        }
        Some(Self {
            range: selected,
            original: successful_source.to_owned(),
        })
    }

    pub fn matches_selection(
        &self,
        selected: Option<ContextRange>,
        has_active_marked: bool,
    ) -> bool {
        !has_active_marked
            && selected == Some(self.range)
            && self.range.length > 0
            && self.range.length <= MAX_REPLACEMENT_UNITS
            && self.range.end().is_some()
    }
}
