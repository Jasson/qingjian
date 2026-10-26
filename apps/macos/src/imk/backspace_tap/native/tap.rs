//! tap 与 run-loop source 的成对释放，所有调用均发生在状态借用之外。

use std::ffi::c_void;

pub struct Tap {
    pub generation: u64,

    pub(super) port: *mut c_void,

    pub(super) source: *mut c_void,
}

impl Drop for Tap {
    fn drop(&mut self) {
        unsafe {
            super::CFRunLoopRemoveSource(
                super::CFRunLoopGetMain(),
                self.source,
                super::kCFRunLoopCommonModes,
            );
            super::CFMachPortInvalidate(self.port);
            super::CFRelease(self.source);
            super::CFRelease(self.port);
        }
    }
}
