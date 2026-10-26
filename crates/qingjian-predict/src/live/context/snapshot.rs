//! 平台采集的实时翻译上下文证据；文本只用于核验，不能直接导入待译历史。

use super::ContextRange;

#[derive(Debug, Clone)]
pub struct ContextSnapshot {
    /// 代理对象标识只用于诊断；同一焦点的代理可能逐次更换，不能据此重置。
    pub client_proxy: usize,

    pub application: Option<String>,

    pub document_length: usize,

    pub selected: Option<ContextRange>,

    pub marked: Option<ContextRange>,

    /// 平台实际发出的前文读取范围；不能把未发请求的空串记为读取成功。
    pub before_range: Option<ContextRange>,

    pub before: Option<String>,

    /// 能力探测的实际请求范围与返回 UTF-16 长度；None 表示宿主拒绝读取。
    pub probe_range: Option<ContextRange>,

    pub probe_length: Option<usize>,

    /// 平台允许的最大回看长度，供区分有限窗口与短局部片段。
    pub lookback_limit: usize,
}

impl ContextSnapshot {
    /// 零长度 marked 只是客户端保留的空范围，位置可能陈旧，不能作为活动组词。
    pub fn active_marked(&self) -> Option<ContextRange> {
        self.marked.filter(|range| range.length > 0)
    }
}
