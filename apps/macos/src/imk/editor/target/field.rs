//! 只读辅助功能身份；不申请权限、不读取文本内容，缺少明确文本框证据则不自动填入。

use objc2_foundation::NSString;
use qingjian_predict::live::context::ContextRange;
use std::ffi::c_void;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXUIElementCreateApplication(pid: i32) -> *const c_void;
    fn AXUIElementCopyAttributeValue(
        element: *const c_void,
        attribute: *const c_void,
        value: *mut *const c_void,
    ) -> i32;
    fn AXUIElementGetTypeID() -> usize;
    fn AXUIElementSetMessagingTimeout(element: *const c_void, timeout: f32) -> i32;
    fn AXUIElementIsAttributeSettable(
        element: *const c_void,
        attribute: *const c_void,
        settable: *mut u8,
    ) -> i32;
    fn AXValueGetTypeID() -> usize;
    fn AXValueGetValue(value: *const c_void, kind: u32, output: *mut c_void) -> u8;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: *const c_void);
    fn CFEqual(first: *const c_void, second: *const c_void) -> u8;
    fn CFGetTypeID(value: *const c_void) -> usize;
    fn CFStringGetTypeID() -> usize;
}

pub(super) struct Field {
    element: *const c_void,

    window: *const c_void,
}

fn attribute(element: *const c_void, name: &str) -> Option<*const c_void> {
    let name = NSString::from_str(name);
    let mut value = std::ptr::null();
    let result = unsafe {
        AXUIElementCopyAttributeValue(
            element,
            &*name as *const NSString as *const c_void,
            &mut value,
        )
    };
    (result == 0 && !value.is_null()).then_some(value)
}

impl Field {
    pub fn capture(pid: i32, expected: ContextRange) -> Option<Self> {
        if pid <= 0 || unsafe { AXIsProcessTrusted() } == 0 {
            return None;
        }
        let app = unsafe { AXUIElementCreateApplication(pid) };
        if app.is_null() {
            return None;
        }
        if unsafe { AXUIElementSetMessagingTimeout(app, 0.1) } != 0 {
            unsafe {
                CFRelease(app);
            }
            return None;
        }
        let element = attribute(app, "AXFocusedUIElement");
        unsafe {
            CFRelease(app);
        }
        let element = element?;
        if unsafe { CFGetTypeID(element) != AXUIElementGetTypeID() } {
            unsafe {
                CFRelease(element);
            }
            return None;
        }
        if unsafe { AXUIElementSetMessagingTimeout(element, 0.1) } != 0 {
            unsafe {
                CFRelease(element);
            }
            return None;
        }
        let selection = attribute(element, "AXSelectedTextRange");
        let selection_matches = selection.is_some_and(|selection| {
            let mut range = [-1isize; 2];
            let valid = unsafe {
                CFGetTypeID(selection) == AXValueGetTypeID()
                    && AXValueGetValue(selection, 4, range.as_mut_ptr().cast()) != 0
            } && range[0] >= 0
                && range[1] >= 0
                && range[0] as usize == expected.start
                && range[1] as usize == expected.length;
            unsafe {
                CFRelease(selection);
            }
            valid
        });
        let value_attribute = NSString::from_str("AXValue");
        let mut settable = 0;
        let editable = unsafe {
            AXUIElementIsAttributeSettable(
                element,
                &*value_attribute as *const NSString as *const c_void,
                &mut settable,
            )
        } == 0
            && settable != 0;
        if !selection_matches || !editable {
            unsafe {
                CFRelease(element);
            }
            return None;
        }
        let role = attribute(element, "AXRole");
        let text_field = role.is_some_and(|role| {
            let valid = unsafe { CFGetTypeID(role) == CFStringGetTypeID() }
                && ["AXTextField", "AXTextArea", "AXComboBox"]
                    .iter()
                    .any(|name| {
                        let name = NSString::from_str(name);
                        unsafe { CFEqual(role, &*name as *const NSString as *const c_void) != 0 }
                    });
            unsafe {
                CFRelease(role);
            }
            valid
        });
        let window = text_field.then(|| attribute(element, "AXWindow")).flatten();
        let Some(window) = window else {
            unsafe {
                CFRelease(element);
            }
            return None;
        };
        if unsafe { CFGetTypeID(window) != AXUIElementGetTypeID() } {
            unsafe {
                CFRelease(element);
                CFRelease(window);
            }
            return None;
        }
        Some(Self { element, window })
    }

    pub fn matches(&self, pid: i32, expected: ContextRange) -> bool {
        Self::capture(pid, expected).is_some_and(|current| unsafe {
            CFEqual(self.element, current.element) != 0 && CFEqual(self.window, current.window) != 0
        })
    }
}

impl Drop for Field {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.element);
            CFRelease(self.window);
        }
    }
}
