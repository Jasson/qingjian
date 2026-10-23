//! Carbon 事件参数中的固定 ABI；ID 使用注册修订号。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub(super) struct HotKeyId {
    pub signature: u32,

    pub id: u32,
}
