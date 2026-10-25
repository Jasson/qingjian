//! 实际 ObjC 派发验证 super 的单次执行、sender/nil 与返回对象身份。

use crate::imk::draft_diagnostics::tests::child::Child;
use objc2::runtime::AnyObject;
use objc2::{ClassType, DeclaredClass, msg_send};
use objc2_foundation::NSObject;

#[test]
fn objc_composition_wrappers_forward_once_and_preserve_sender_and_raw_return_identity() {
    let receiver = Child::new();
    let sender = NSObject::new();
    let returned = NSObject::new();
    let returned_pointer = std::ptr::from_ref::<AnyObject>(&returned).cast_mut();
    let state = receiver.as_super().ivars();
    state.result.set(returned_pointer);
    let actual: *mut AnyObject = unsafe { msg_send![&*receiver, composedString: &*sender] };
    assert_eq!(actual, returned_pointer);
    assert_eq!(state.sender.get(), std::ptr::from_ref::<AnyObject>(&sender));
    let actual: *mut AnyObject =
        unsafe { msg_send![&*receiver, originalString: std::ptr::null::<AnyObject>()] };
    assert_eq!(actual, returned_pointer);
    assert!(state.sender.get().is_null());
    unsafe {
        let _: () = msg_send![&*receiver, updateComposition];
        let _: () = msg_send![&*receiver, cancelComposition];
    }
    assert_eq!(state.counts.get(), [1, 1, 1, 1]);
    state.result.set(std::ptr::null_mut());
    let actual: *mut AnyObject =
        unsafe { msg_send![&*receiver, composedString: std::ptr::null::<AnyObject>()] };
    assert!(actual.is_null());
    assert_eq!(state.counts.get(), [2, 1, 1, 1]);
    assert!(state.sender.get().is_null());
}
