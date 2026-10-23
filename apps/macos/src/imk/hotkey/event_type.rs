//! Carbon 事件监听类型的固定 ABI。

#[repr(C)]
pub(super) struct EventType {
    pub class: u32,

    pub kind: u32,
}
