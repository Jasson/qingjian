//! 平台声明的文档核验策略；事件模式不把显示文本视为可编辑文档。

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ContextPolicy {
    #[default]
    VerifiedDocument,

    /// 只累积受控输入事件；宿主回显仅供诊断。
    EventOnly,
}
