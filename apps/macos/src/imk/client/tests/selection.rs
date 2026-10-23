//! 记录客户端写入，验证空缓冲不会覆盖用户选区。

use objc2::rc::Retained;
use objc2::{AnyThread, DeclaredClass, define_class, msg_send};
use objc2_foundation::{NSObject, NSObjectProtocol, NSRange, NSString};
use std::cell::RefCell;

define_class!(
    // SAFETY: NSObject 允许子类化；仅在测试当前线程访问，不跨线程共享。
    #[unsafe(super(NSObject))]
    #[name = "QingjianTestSelectionClient"]
    #[ivars = RefCell<Vec<String>>]
    pub(super) struct SelectionClient;

    impl SelectionClient {
        #[unsafe(method(length))]
        fn length(&self) -> usize { 0 }

        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange { NSRange::new(0, 0) }

        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange { NSRange::new(usize::MAX, 0) }

        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self, text: &NSString, _range: NSRange) {
            self.ivars().borrow_mut().push(format!("insert:{}", text));
        }

        #[unsafe(method(setMarkedText:selectionRange:replacementRange:))]
        fn set_marked_text(&self, text: &NSString, _selection: NSRange, _replacement: NSRange) {
            self.ivars().borrow_mut().push(format!("mark:{}", text));
        }
    }

    unsafe impl NSObjectProtocol for SelectionClient {}
);

impl SelectionClient {
    pub(super) fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(RefCell::new(Vec::new()));
        unsafe { msg_send![super(this), init] }
    }
}
