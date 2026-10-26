//! 主线程 Carbon 热键薄封装；只使用排他注册，不回退到共享组合键。

use std::ffi::c_void;
use std::ptr;

use super::binding::Binding;
use super::event_type::EventType;
use super::id::HotKeyId;

pub(super) const SIGNATURE: u32 = u32::from_be_bytes(*b"QjLT");
const KEYBOARD: u32 = u32::from_be_bytes(*b"keyb");
const HOT_KEY_TYPE: u32 = u32::from_be_bytes(*b"hkid");
const DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");
const PRESSED: u32 = 5;
const RELEASED: u32 = 6;
const EXCLUSIVE: u32 = 1;
const NOT_HANDLED: i32 = -9874;

type Callback = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn GetApplicationEventTarget() -> *mut c_void;
    fn InstallEventHandler(
        target: *mut c_void,
        handler: Callback,
        count: usize,
        types: *const EventType,
        data: *mut c_void,
        result: *mut *mut c_void,
    ) -> i32;
    fn RemoveEventHandler(handler: *mut c_void) -> i32;
    fn RegisterEventHotKey(
        key: u32,
        modifiers: u32,
        id: HotKeyId,
        target: *mut c_void,
        options: u32,
        result: *mut *mut c_void,
    ) -> i32;
    fn UnregisterEventHotKey(hotkey: *mut c_void) -> i32;
    fn GetEventClass(event: *mut c_void) -> u32;
    fn GetEventKind(event: *mut c_void) -> u32;
    fn GetEventParameter(
        event: *mut c_void,
        name: u32,
        desired_type: u32,
        actual_type: *mut u32,
        size: usize,
        actual_size: *mut usize,
        data: *mut c_void,
    ) -> i32;
}

pub(super) fn install() -> Result<usize, i32> {
    let types = [
        EventType {
            class: KEYBOARD,
            kind: PRESSED,
        },
        EventType {
            class: KEYBOARD,
            kind: RELEASED,
        },
    ];
    let mut handler = ptr::null_mut();
    let status = unsafe {
        InstallEventHandler(
            GetApplicationEventTarget(),
            callback,
            types.len(),
            types.as_ptr(),
            ptr::null_mut(),
            &mut handler,
        )
    };
    if status != 0 || handler.is_null() {
        if !handler.is_null() {
            unsafe {
                RemoveEventHandler(handler);
            }
        }
        Err(if status == 0 { -50 } else { status })
    } else {
        Ok(handler as usize)
    }
}

pub(super) fn register(binding: Binding, generation: u32) -> Result<usize, i32> {
    let key = binding.key_code().ok_or(-50)?;
    let mut handle = ptr::null_mut();
    let status = unsafe {
        RegisterEventHotKey(
            key,
            binding.modifiers(),
            HotKeyId {
                signature: SIGNATURE,
                id: generation,
            },
            GetApplicationEventTarget(),
            EXCLUSIVE,
            &mut handle,
        )
    };
    if status != 0 || handle.is_null() {
        if !handle.is_null() {
            unsafe {
                UnregisterEventHotKey(handle);
            }
        }
        Err(if status == 0 { -50 } else { status })
    } else {
        Ok(handle as usize)
    }
}

pub(super) fn unregister(handle: usize) -> Result<(), i32> {
    let status = unsafe { UnregisterEventHotKey(handle as *mut c_void) };
    if status == 0 { Ok(()) } else { Err(status) }
}

pub(super) fn event_id(event: *mut c_void) -> Option<(HotKeyId, bool)> {
    if event.is_null() {
        return None;
    }
    let class = unsafe { GetEventClass(event) };
    let kind = unsafe { GetEventKind(event) };
    if class != KEYBOARD || !matches!(kind, PRESSED | RELEASED) {
        return None;
    }
    let mut id = HotKeyId {
        signature: 0,
        id: 0,
    };
    let mut actual_type = 0;
    let mut actual_size = 0;
    let status = unsafe {
        GetEventParameter(
            event,
            DIRECT_OBJECT,
            HOT_KEY_TYPE,
            &mut actual_type,
            size_of::<HotKeyId>(),
            &mut actual_size,
            (&mut id as *mut HotKeyId).cast(),
        )
    };
    (status == 0 && actual_type == HOT_KEY_TYPE && actual_size == size_of::<HotKeyId>())
        .then_some((id, kind == PRESSED))
}

unsafe extern "C" fn callback(
    _handler: *mut c_void,
    event: *mut c_void,
    _data: *mut c_void,
) -> i32 {
    // FFI 边界绝不展开；回调仅排队，不在 Carbon 栈中借 Host 或写入应用。
    std::panic::catch_unwind(|| {
        if objc2::MainThreadMarker::new().is_none() {
            return NOT_HANDLED;
        }
        let Some((id, pressed)) = event_id(event) else {
            return NOT_HANDLED;
        };
        if id.signature != SIGNATURE {
            return NOT_HANDLED;
        }
        super::receive(id.id, pressed);
        0
    })
    .unwrap_or(NOT_HANDLED)
}
