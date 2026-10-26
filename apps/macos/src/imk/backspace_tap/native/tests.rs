//! 用本地构造、从未 Post 的 CGEvent 核验生产解码 ABI。

use std::ffi::c_void;
use std::ptr;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateKeyboardEvent(state: *const c_void, key: u16, down: bool) -> *mut c_void;
    fn CGEventSetIntegerValueField(event: *mut c_void, field: u32, value: i64);
    fn CGEventSetTimestamp(event: *mut c_void, timestamp: u64);
    fn CGEventSetFlags(event: *mut c_void, flags: u64);
}

#[test]
fn native_local_event_decodes_only_metadata_with_the_production_abi() {
    for key in [51, 117] {
        let event = unsafe { CGEventCreateKeyboardEvent(ptr::null(), key, true) };
        assert!(!event.is_null());
        unsafe {
            CGEventSetIntegerValueField(event, 8, 1);
            CGEventSetIntegerValueField(event, 40, 123);
            CGEventSetTimestamp(event, 9_000_000);
            CGEventSetFlags(event, 1 << 23);
        }
        let metadata = super::metadata(10, event, 7).unwrap();
        unsafe { super::CFRelease(event) };
        assert_eq!(metadata.key, i64::from(key));
        assert_eq!(metadata.pid, 123);
        assert_eq!(metadata.generation, 7);
        assert_eq!(metadata.timestamp, 9_000_000);
        assert_eq!(metadata.flags, 1 << 23);
        assert!(metadata.repeat);
    }
}

#[test]
fn native_disabled_notification_needs_no_event_and_null_normal_events_are_ignored() {
    assert!(super::metadata(10, ptr::null_mut(), 7).is_none());
    for kind in [0xffff_fffe, 0xffff_ffff] {
        let metadata = super::metadata(kind, ptr::null_mut(), 7).unwrap();
        assert_eq!(metadata.kind, kind);
        assert_eq!(metadata.key, 0);
    }
}
