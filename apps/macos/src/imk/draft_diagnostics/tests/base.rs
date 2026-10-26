//! 模拟已经实现四个组句方法的父类，记录真正收到的 ObjC 调用。

use crate::imk::draft_diagnostics::tests::state::MockState;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::AnyObject;
use objc2::{DeclaredClass, define_class, msg_send};
use objc2_foundation::{NSObject, NSObjectProtocol};

define_class!(
    // SAFETY: NSObject 的普通测试子类；状态不跨线程访问。
    #[unsafe(super(NSObject))]
    #[name = "QingjianDiagnosticForwardBase"]
    #[ivars = MockState]
    pub(super) struct Base;

    impl Base {
        #[unsafe(method_id(init))]
        fn init(this: Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(MockState::default());
            unsafe { msg_send![super(this), init] }
        }

        #[unsafe(method(composedString:))]
        fn composed(&self, sender: Option<&AnyObject>) -> *mut AnyObject {
            self.note(0, sender);
            self.ivars().result.get()
        }

        #[unsafe(method(originalString:))]
        fn original(&self, sender: Option<&AnyObject>) -> *mut AnyObject {
            self.note(1, sender);
            self.ivars().result.get()
        }

        #[unsafe(method(updateComposition))]
        fn update(&self) { self.note(2, None); }

        #[unsafe(method(cancelComposition))]
        fn cancel(&self) { self.note(3, None); }
    }

    unsafe impl NSObjectProtocol for Base {}
);

impl Base {
    fn note(&self, index: usize, sender: Option<&AnyObject>) {
        let mut counts = self.ivars().counts.get();
        counts[index] += 1;
        self.ivars().counts.set(counts);
        self.ivars()
            .sender
            .set(sender.map_or(std::ptr::null(), std::ptr::from_ref));
    }
}
