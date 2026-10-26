//! 写口计数：sent 仅表示即将请求原有 IPC，不代表宿主已接收或写入成功。

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct WriteCounts {
    pub attempted: u16,

    pub blocked: u16,

    pub sent: u16,
}
