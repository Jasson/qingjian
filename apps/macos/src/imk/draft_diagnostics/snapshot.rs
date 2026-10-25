//! 不读取客户端文本的 Host 缓存快照；借用失败以 None 表示。

#[derive(Clone, Copy, Debug)]
pub(crate) struct Snapshot {
    pub wechat: bool,

    pub staged: bool,

    pub epoch: u64,

    pub composing: bool,

    pub draft_present: bool,
}
