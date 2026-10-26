//! CoreGraphics ForPid active tap 的最小 ABI；不申请权限、不发送事件。

mod tap;
#[cfg(test)]
mod tests;

use std::ffi::c_void;
use std::ptr;

use super::event::Event;
pub use tap::Tap;

type Callback = unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void) -> *mut c_void;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventTapCreateForPid(
        pid: i32,
        place: u32,
        options: u32,
        mask: u64,
        callback: Callback,
        data: *mut c_void,
    ) -> *mut c_void;
    fn CGEventGetIntegerValueField(event: *mut c_void, field: u32) -> i64;
    fn CGEventGetTimestamp(event: *mut c_void) -> u64;
    fn CGEventGetFlags(event: *mut c_void) -> u64;
    fn CGPreflightListenEventAccess() -> bool;
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: *mut c_void,
        order: isize,
    ) -> *mut c_void;
    fn CFMachPortInvalidate(port: *mut c_void);
    fn CFRunLoopGetMain() -> *mut c_void;
    fn CFRunLoopAddSource(loop_: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRemoveSource(loop_: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRelease(value: *const c_void);
    static kCFRunLoopCommonModes: *const c_void;
}

pub fn create(pid: i32, generation: u64) -> Option<Tap> {
    let (accessibility, listening) =
        unsafe { (AXIsProcessTrusted() != 0, CGPreflightListenEventAccess()) };
    let port = unsafe {
        CGEventTapCreateForPid(
            pid,
            0,
            0,
            (1 << 10) | (1 << 11),
            callback,
            generation as usize as *mut c_void,
        )
    };
    if port.is_null() {
        tracing::warn!(target: "live_translation", accessibility, listening,
            "草稿退格接管创建失败，请检查青简的辅助功能与输入监控权限");
        return None;
    }
    let source = unsafe { CFMachPortCreateRunLoopSource(ptr::null(), port, 0) };
    if source.is_null() {
        unsafe {
            CFMachPortInvalidate(port);
            CFRelease(port);
        }
        return None;
    }
    unsafe { CFRunLoopAddSource(CFRunLoopGetMain(), source, kCFRunLoopCommonModes) };
    Some(Tap {
        generation,
        port,
        source,
    })
}

unsafe extern "C" fn callback(
    _proxy: *mut c_void,
    kind: u32,
    event: *mut c_void,
    data: *mut c_void,
) -> *mut c_void {
    // 同一主 run-loop 回调只排队；不读 Host、NSWorkspace、客户或 Unicode。
    std::panic::catch_unwind(|| {
        if objc2::MainThreadMarker::new().is_none() {
            return event;
        }
        let Some(metadata) = metadata(kind, event, data as usize as u64) else {
            return event;
        };
        if super::receive(metadata) {
            ptr::null_mut()
        } else {
            event
        }
    })
    .unwrap_or(event)
}

fn metadata(kind: u32, event: *mut c_void, generation: u64) -> Option<Event> {
    let disabled = matches!(kind, 0xffff_fffe | 0xffff_ffff);
    if event.is_null() && !disabled {
        return None;
    }
    Some(Event {
        generation,
        kind,
        key: if disabled {
            0
        } else {
            unsafe { CGEventGetIntegerValueField(event, 9) }
        },
        flags: if disabled {
            0
        } else {
            unsafe { CGEventGetFlags(event) }
        },
        pid: if disabled {
            0
        } else {
            unsafe { CGEventGetIntegerValueField(event, 40) }
        },
        timestamp: if disabled {
            0
        } else {
            unsafe { CGEventGetTimestamp(event) }
        },
        repeat: !disabled && unsafe { CGEventGetIntegerValueField(event, 8) != 0 },
        secure: crate::imk::secure_input::enabled(),
    })
}
