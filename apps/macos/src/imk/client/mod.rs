//! 对 IMK 客户端对象（实现 `IMKTextInput` 协议的代理）的薄封装。
//!
//! objc2-input-method-kit 没有为 IMKTextInput 生成绑定，这里用 `msg_send!` 直接发消息。

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2::{msg_send, sel};
use objc2_foundation::{
    NSAttributedString, NSDictionary, NSNotFound, NSPoint, NSRange, NSRect, NSString,
};
use qingjian_core::SurroundingText;
use qingjian_predict::live::context::{ContextRange, ContextSnapshot};

mod bounded_read;
pub use bounded_read::BoundedRead;

/// `{NSNotFound, 0}`：不指定替换范围，由客户端使用当前选区或 marked text。
const NO_REPLACEMENT: NSRange = NSRange::new(NSNotFound as usize, 0);

#[derive(Clone, Copy)]
pub struct TextClient<'a> {
    /// IMK 传进来的 `sender`。
    object: &'a AnyObject,

    stage_output: bool,
}

impl<'a> TextClient<'a> {
    pub fn new(object: &'a AnyObject) -> Self {
        Self {
            object,
            stage_output: crate::host::stage_text_output(),
        }
    }

    /// 设置 marked text（带下划线的未上屏文本），光标放在第 `cursor` 个字符处。空串等于清除。
    pub fn set_marked_text(&self, text: &str, cursor: usize) {
        if self.stages_output() {
            return;
        }
        let string = NSString::from_str(text);
        let cursor = NSRange::new(cursor.min(text.chars().count()), 0);
        unsafe {
            let _: () = msg_send![
                self.object,
                setMarkedText: &*string,
                selectionRange: cursor,
                replacementRange: NO_REPLACEMENT
            ];
        }
    }

    /// IMK 客户端按屏幕坐标判断点击是否落在 marked text 内，保留内部点击行为。
    pub fn point_in_marked_text(&self, point: NSPoint) -> bool {
        let mut inside = Bool::NO;
        // IMKInputSession.h：kIMKNearestBoundaryMode = 1 << 1；非跟踪模式，不改变选区。
        let index: isize = unsafe {
            msg_send![self.object, characterIndexForPoint: point, tracking: 2isize, inMarkedRange: &mut inside]
        };
        index != NSNotFound && inside.as_bool()
    }

    /// 切换模式时仅提交非空组句。空缓冲不能给客户端发文本写入，否则会替换当前选区。
    pub fn commit_pending_raw(&self, raw: &str) {
        if !raw.is_empty() {
            // insertText 已解除 marked text；不能再 setMarkedText 空串，以免重入后误删新选区。
            self.commit_text(raw);
        }
    }

    /// 统一的已确认文本路径：先更新受控输入状态，再经写入门禁决定是否发送给宿主。
    pub fn commit_text(&self, text: &str) {
        self.commit_with(text, |text| {
            crate::host::with(|h| h.commit_live_text(text));
        });
    }

    fn commit_with(&self, text: &str, stage: impl FnOnce(&str)) {
        stage(text);
        self.insert_text(text);
    }

    /// 上屏。
    pub fn insert_text(&self, text: &str) {
        if self.stages_output() {
            return;
        }
        self.insert_unchecked(text);
    }

    pub fn stages_output(&self) -> bool {
        self.stage_output || crate::host::stage_text_output()
    }

    /// 唯一草稿确认写入口；调用方须先核验并消耗对应草稿，不能借此提交普通原文。
    pub fn insert_confirmed_draft(&self, text: &str) -> bool {
        if qingjian_predict::live::normalize_terminal_output(text).as_deref() != Some(text)
            || text.trim().is_empty()
        {
            return false;
        }
        self.insert_unchecked(text);
        true
    }

    fn insert_unchecked(&self, text: &str) {
        let string = NSString::from_str(text);
        unsafe {
            let _: () =
                msg_send![self.object, insertText: &*string, replacementRange: NO_REPLACEMENT];
        }
    }

    /// 应用里当前选中的文字与它的范围（翻译用）。没有选区、应用不支持读文本、超过 `max_chars` 个字符都返回 `None`。
    pub fn selected_text(&self, max_chars: usize) -> Option<(String, NSRange)> {
        let selected: NSRange = unsafe { msg_send![self.object, selectedRange] };
        if selected.location == NSNotFound as usize
            || selected.length == 0
            || selected.length > max_chars
        {
            return None;
        }
        let text: Option<Retained<NSAttributedString>> =
            unsafe { msg_send![self.object, attributedSubstringFromRange: selected] };
        let text = text?.string().to_string();
        (!text.trim().is_empty()).then_some((text, selected))
    }

    /// 读取当前应用中的精确范围；无效范围绝不发送给客户端。
    #[cfg(test)]
    pub fn read_range(&self, range: NSRange) -> Option<String> {
        let length: usize = unsafe { msg_send![self.object, length] };
        if range.location == NSNotFound as usize
            || range
                .location
                .checked_add(range.length)
                .is_none_or(|end| end > length)
        {
            return None;
        }
        let text: Option<Retained<NSAttributedString>> =
            unsafe { msg_send![self.object, attributedSubstringFromRange: range] };
        text.map(|text| text.string().to_string())
    }

    /// 用 `text` 替换应用里 `range` 那段文字（翻译结果替换选区）。
    pub fn replace_range(&self, text: &str, range: NSRange) {
        if self.stages_output() {
            return;
        }
        let string = NSString::from_str(text);
        unsafe {
            let _: () = msg_send![self.object, insertText: &*string, replacementRange: range];
        }
    }

    /// 读光标附近的文本给联想当上下文：marked text 之前 `before` 个字符、之后 `after` 个字符。
    /// 应用不支持 `attributedSubstringFromRange:`（不少 Electron / 终端）时返回 `None`，由 Core 退回本地历史。
    pub fn surrounding_text(&self, before: usize, after: usize) -> Option<SurroundingText> {
        let (length, selected, marked): (usize, NSRange, NSRange) = unsafe {
            (
                msg_send![self.object, length],
                msg_send![self.object, selectedRange],
                msg_send![self.object, markedRange],
            )
        };
        if selected.location == NSNotFound as usize {
            return None;
        }
        if length == 0 {
            return (selected.location == 0 && selected.length == 0).then(|| SurroundingText {
                before: String::new(),
                after: String::new(),
            });
        }
        // 组句中光标在 marked text 里；上下文以 marked text 为界
        let (start, end) = if marked.location == NSNotFound as usize {
            (selected.location, selected.location + selected.length)
        } else {
            (marked.location, marked.location + marked.length)
        };
        let start = start.min(length);
        let end = end.min(length);
        let before_range = NSRange::new(
            start.saturating_sub(before),
            start - start.saturating_sub(before),
        );
        let after_range = NSRange::new(end, after.min(length - end));
        let read = |range: NSRange| -> Option<String> {
            if range.length == 0 {
                return Some(String::new());
            }
            let text: Option<Retained<NSAttributedString>> =
                unsafe { msg_send![self.object, attributedSubstringFromRange: range] };
            text.map(|t| t.string().to_string())
        };
        Some(SurroundingText {
            before: read(before_range)?,
            after: read(after_range)?,
        })
    }

    /// 手动替换只采集当前选区与 marked 元数据，不请求整段前文。
    pub fn live_selection_metadata(
        &self,
        still_current: impl Fn() -> bool,
    ) -> Option<ContextSnapshot> {
        if !still_current() {
            return None;
        }
        let length: usize = unsafe { msg_send![self.object, length] };
        if !still_current() {
            return None;
        }
        let selected: NSRange = unsafe { msg_send![self.object, selectedRange] };
        if !still_current() {
            return None;
        }
        let marked: NSRange = unsafe { msg_send![self.object, markedRange] };
        if !still_current() {
            return None;
        }
        let application = self.bundle_identifier();
        if !still_current() {
            return None;
        }
        let selected = (selected.location < NSNotFound as usize
            && selected
                .location
                .checked_add(selected.length)
                .is_some_and(|end| end < NSNotFound as usize))
        .then_some(ContextRange {
            start: selected.location,
            length: selected.length,
        });
        let marked = (marked.location != NSNotFound as usize).then_some(ContextRange {
            start: marked.location,
            length: marked.length,
        });
        Some(ContextSnapshot {
            client_proxy: self.object as *const AnyObject as usize,
            application,
            document_length: length,
            selected,
            marked,
            before_range: None,
            before: None,
            probe_range: None,
            probe_length: None,
            lookback_limit: 0,
        })
    }

    /// IMK 文档访问属性明确承诺支持绝对 replacementRange，不能由读回成功代替。
    pub fn supports_document_access(&self, still_current: impl Fn() -> bool) -> bool {
        if !still_current() {
            return false;
        }
        let responds: bool =
            unsafe { msg_send![self.object, respondsToSelector: sel!(supportsProperty:)] };
        if !still_current() || !responds {
            return false;
        }
        let supports: bool =
            unsafe { msg_send![self.object, supportsProperty: u32::from_be_bytes(*b"dapy")] };
        still_current() && supports
    }

    /// 仅读取当前明确非空选区；宿主 length 不作为支持读取的证据。
    pub fn read_selected_candidate(
        &self,
        candidate: &qingjian_predict::live::ReplacementCandidate,
        metadata: &ContextSnapshot,
        still_current: impl Fn() -> bool,
    ) -> BoundedRead {
        self.read_bounded_candidate(candidate, metadata, still_current, false)
    }

    #[cfg(test)]
    pub fn read_caret_bounded_candidate(
        &self,
        candidate: &qingjian_predict::live::ReplacementCandidate,
        metadata: &ContextSnapshot,
        still_current: impl Fn() -> bool,
    ) -> BoundedRead {
        self.read_bounded_candidate(candidate, metadata, still_current, true)
    }

    fn read_bounded_candidate(
        &self,
        candidate: &qingjian_predict::live::ReplacementCandidate,
        metadata: &ContextSnapshot,
        still_current: impl Fn() -> bool,
        caret_test_adapter: bool,
    ) -> BoundedRead {
        let mut result = BoundedRead {
            text: None,
            reader: "none",
            attempted: false,
        };
        let Some(selected) = metadata.selected else {
            return result;
        };
        let range = candidate.range();
        if metadata.client_proxy != self.object as *const AnyObject as usize
            || selected.start >= NSNotFound as usize
            || selected.end().is_none_or(|end| end >= NSNotFound as usize)
            || if caret_test_adapter {
                !candidate.matches_caret(Some(selected), metadata.active_marked().is_some())
                    || range.end() != Some(selected.start)
            } else {
                !candidate.matches_selection(Some(selected), metadata.active_marked().is_some())
                    || range != selected
            }
            || !still_current()
        {
            return result;
        }
        let native_range = NSRange::new(range.start, range.length);
        result.reader = "legacy";
        result.attempted = true;
        let text: Option<Retained<NSAttributedString>> =
            unsafe { msg_send![self.object, attributedSubstringFromRange: native_range] };
        if !still_current() {
            return result;
        }
        if let Some(text) = text {
            let text = text.string().to_string();
            if still_current() {
                result.text = Some(text);
            }
            return result;
        }
        // 优先使用 IMKTextInput 公开的带实际范围读取，不假定代理暴露 NSTextInputClient。
        let imk_supported: bool = unsafe {
            msg_send![self.object, respondsToSelector: sel!(stringFromRange:actualRange:)]
        };
        if !still_current() {
            return result;
        }
        if imk_supported {
            result.reader = "imk";
            let mut actual = NSRange::new(NSNotFound as usize, 0);
            let text: Option<Retained<NSString>> = unsafe {
                msg_send![self.object, stringFromRange: native_range, actualRange: &mut actual]
            };
            if !still_current() {
                return result;
            }
            if let Some(text) = text {
                let text = text.to_string();
                if still_current() && actual == native_range {
                    result.text = Some(text);
                }
                return result;
            }
        }

        result
    }

    /// 实时翻译专用：保留范围与实际读取证据，不能由零范围直接推断文档为空。
    pub fn live_context(&self, lookback: usize) -> ContextSnapshot {
        let (length, selected, marked): (usize, NSRange, NSRange) = unsafe {
            (
                msg_send![self.object, length],
                msg_send![self.object, selectedRange],
                msg_send![self.object, markedRange],
            )
        };
        let convert = |range: NSRange| {
            (range.location != NSNotFound as usize).then_some(ContextRange {
                start: range.location,
                length: range.length,
            })
        };
        let selected = convert(selected);
        let marked = convert(marked);
        let mut snapshot = ContextSnapshot {
            client_proxy: self.object as *const AnyObject as usize,
            application: self.bundle_identifier(),
            document_length: length,
            selected,
            marked,
            before_range: None,
            before: None,
            probe_range: None,
            probe_length: None,
            lookback_limit: lookback,
        };
        let active_marked = snapshot.active_marked();
        let valid = selected.is_some_and(|range| range.within(length))
            && active_marked.is_none_or(|range| range.within(length));
        let start = active_marked.or(selected).map(|range| range.start);
        let before_range = valid.then(|| ContextRange {
            start: start.unwrap_or(0).saturating_sub(lookback),
            length: start.unwrap_or(0).min(lookback),
        });
        let read = |range: ContextRange| -> Option<String> {
            let attributed: Option<Retained<NSAttributedString>> = unsafe {
                msg_send![self.object, attributedSubstringFromRange: NSRange::new(range.start, range.length)]
            };
            attributed.map(|text| text.string().to_string())
        };
        let before = before_range.and_then(read);
        // 非空前文读取本身就是能力证据；空前文再读 marked，真正空文档才用零范围探测。
        let probe_range = before_range.map(|before| {
            if before.length > 0 {
                before
            } else {
                active_marked
                    .filter(|range| range.length <= lookback)
                    .unwrap_or(ContextRange {
                        start: 0,
                        length: 0,
                    })
            }
        });
        let probe_length = if probe_range == before_range {
            before.as_ref().map(|text| text.encode_utf16().count())
        } else {
            probe_range
                .and_then(read)
                .map(|text| text.encode_utf16().count())
        };
        snapshot.before_range = before_range;
        snapshot.before = before;
        snapshot.probe_range = probe_range;
        snapshot.probe_length = probe_length;
        snapshot
    }

    /// 正在输入的应用的 bundle identifier（`com.apple.Terminal`），按应用改行为用；应用没给返回 `None`。
    pub fn bundle_identifier(&self) -> Option<String> {
        let bundle: Option<Retained<NSString>> =
            unsafe { msg_send![self.object, bundleIdentifier] };
        bundle.map(|b| b.to_string()).filter(|b| !b.is_empty())
    }

    /// 光标（marked text 起点）所在行在屏幕坐标系里的矩形，用来定位候选窗口。
    /// 应用不支持时返回零矩形，窗口就会落在屏幕左下角，至少看得见。
    pub fn caret_rect(&self) -> NSRect {
        let mut rect = NSRect::ZERO;
        unsafe {
            let _: Option<Retained<NSDictionary>> = msg_send![self.object, attributesForCharacterIndex: 0usize, lineHeightRectangle: &mut rect];
        }
        rect
    }
}

#[cfg(test)]
mod tests;
