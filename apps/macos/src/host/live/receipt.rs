//! 成功译文的来源凭据；替换操作必须绑定当前请求与会话。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuccessReceipt {
    pub text: String,

    pub source: String,

    pub request_id: u64,

    pub epoch: u64,
}
