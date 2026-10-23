//! 可按范围返回文本或拒绝读取的 IMK 客户端替身。

use super::state::ContextState;
use objc2::rc::Retained;
use objc2::{AnyThread, DeclaredClass, define_class, msg_send};
use objc2_foundation::{NSAttributedString, NSObject, NSObjectProtocol, NSRange, NSString};
use std::cell::{Cell, RefCell};

define_class!(
    // SAFETY: NSObject 允许子类化；测试对象仅在创建线程使用。
    #[unsafe(super(NSObject))]
    #[name = "QingjianTestLiveContextClient"]
    #[ivars = ContextState]
    pub(super) struct ContextClient;

    impl ContextClient {
        #[unsafe(method(length))]
        fn length(&self) -> usize { self.ivars().length }

        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange { self.ivars().selected.get() }

        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange { self.ivars().marked.get() }

        #[unsafe(method_id(bundleIdentifier))]
        fn bundle(&self) -> Option<Retained<NSString>> { Some(NSString::from_str("test.client")) }

        #[unsafe(method(supportsProperty:))]
        fn supports_property(&self, property: u32) -> bool {
            self.ivars().support_queries.borrow_mut().push(property);
            property == u32::from_be_bytes(*b"dapy") && self.ivars().document_access.get()
        }

        #[unsafe(method_id(stringFromRange:actualRange:))]
        fn string_from_range(&self, range: NSRange, actual: *mut NSRange) -> Option<Retained<NSString>> {
            self.read_imk_string(range, actual)
        }

        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self, text: &NSString, range: NSRange) {
            self.ivars().writes.borrow_mut().push((text.to_string(), range));
        }

        #[unsafe(method_id(attributedSubstringFromRange:))]
        fn substring(&self, range: NSRange) -> Option<Retained<NSAttributedString>> {
            self.read_substring(range)
        }
    }

    unsafe impl NSObjectProtocol for ContextClient {}
);

impl ContextClient {
    fn read_text(&self, range: NSRange) -> Option<String> {
        self.ivars().requests.borrow_mut().push(range);
        if range.length > self.ivars().maximum_read_units.get() {
            return None;
        }
        let text: Vec<u16> = self.ivars().text.as_ref()?.encode_utf16().collect();
        let end = range.location.checked_add(range.length)?;
        String::from_utf16(text.get(range.location..end)?).ok()
    }

    fn read_substring(&self, range: NSRange) -> Option<Retained<NSAttributedString>> {
        if !self.ivars().legacy_readable.get() {
            self.ivars().requests.borrow_mut().push(range);
            return None;
        }
        let string = NSString::from_str(&self.read_text(range)?);
        Some(unsafe { msg_send![NSAttributedString::alloc(), initWithString: &*string] })
    }

    fn read_imk_string(&self, range: NSRange, actual: *mut NSRange) -> Option<Retained<NSString>> {
        let text = self.read_text(range)?;
        if !actual.is_null() {
            unsafe {
                *actual = if self.ivars().adjust_actual.get() {
                    NSRange::new(range.location + 1, range.length)
                } else {
                    range
                };
            }
        }
        Some(NSString::from_str(&text))
    }

    pub(super) fn new(
        text: Option<&str>,
        length: usize,
        selected: NSRange,
        marked: NSRange,
    ) -> Retained<Self> {
        let this = Self::alloc().set_ivars(ContextState {
            text: text.map(str::to_owned),
            length,
            selected: Cell::new(selected),
            marked: Cell::new(marked),
            requests: RefCell::new(Vec::new()),
            writes: RefCell::new(Vec::new()),
            document_access: Cell::new(true),
            support_queries: RefCell::new(Vec::new()),
            legacy_readable: Cell::new(true),
            maximum_read_units: Cell::new(usize::MAX),
            adjust_actual: Cell::new(false),
        });
        unsafe { msg_send![super(this), init] }
    }

    pub(super) fn writes(&self) -> Vec<(String, NSRange)> {
        self.ivars().writes.borrow().clone()
    }

    pub(super) fn requests(&self) -> Vec<NSRange> {
        self.ivars().requests.borrow().clone()
    }
}
