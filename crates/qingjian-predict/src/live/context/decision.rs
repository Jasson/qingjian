//! 上下文核验结果；原因使用固定标识，不携带用户文本。

use super::ContextConfidence;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextDecision {
    pub confidence: ContextConfidence,

    /// 显示文本与受控历史不符；事件模式下仅供诊断。
    pub context_mismatch: bool,

    pub reset_reason: Option<&'static str>,
}
