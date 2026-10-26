//! 使用与生产完全相同的 callback 包装与 super 原始对象指针透传。

use crate::imk::draft_diagnostics::callback;
use crate::imk::draft_diagnostics::tests::base::Base;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread, define_class, msg_send};
use objc2_foundation::NSObjectProtocol;

define_class!(
    // SAFETY: Base 的普通子类；只包装已有方法，不新增输入事件协议。
    #[unsafe(super(Base))]
    #[name = "QingjianDiagnosticForwardChild"]
    #[ivars = ()]
    pub(super) struct Child;

    impl Child {
        #[unsafe(method(composedString:))]
        fn composed(&self, sender: Option<&AnyObject>) -> *mut AnyObject {
            callback("composedString", || true, || unsafe { msg_send![super(self), composedString: sender] })
        }

        #[unsafe(method(originalString:))]
        fn original(&self, sender: Option<&AnyObject>) -> *mut AnyObject {
            callback("originalString", || true, || unsafe { msg_send![super(self), originalString: sender] })
        }

        #[unsafe(method(updateComposition))]
        fn update(&self) {
            callback("updateComposition", || true, || unsafe { let _: () = msg_send![super(self), updateComposition]; });
        }

        #[unsafe(method(cancelComposition))]
        fn cancel(&self) {
            callback("cancelComposition", || true, || unsafe { let _: () = msg_send![super(self), cancelComposition]; });
        }
    }

    unsafe impl NSObjectProtocol for Child {}
);

impl Child {
    pub(super) fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
