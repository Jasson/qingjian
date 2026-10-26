//! 事件 tap 只读取按键路由元数据，不读取字符。

#[derive(Clone, Copy)]
pub(super) struct Event {
    pub generation: u64,

    pub kind: u32,

    pub key: i64,

    pub flags: u64,

    pub pid: i64,

    pub timestamp: u64,

    pub repeat: bool,

    pub secure: bool,
}
