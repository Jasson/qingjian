//! 实时翻译可见前文的可信度。

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ContextConfidence {
    /// 范围与实际读取吻合，能验证当前受控历史或完整的有限回看窗口。
    Full,

    /// 只看得到局部片段，无法据此否定受控历史。
    Partial,

    /// 元数据无效、实际读取失败或返回长度与请求不符。
    #[default]
    Unreadable,
}
