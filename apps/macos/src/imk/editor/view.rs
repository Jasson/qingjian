//! 自有第一响应者直接处理按键；不建立 NSTextInputContext 或触发本输入法递归。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize};

define_class!(
    // SAFETY: NSView 的主线程子类；所有输入都交独立编辑器，不调用 interpretKeyEvents。
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    pub(super) struct EditorView;

    impl EditorView {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool { true }

        #[unsafe(method(needsPanelToBecomeKey))]
        fn needs_panel_key(&self) -> bool { true }

        #[unsafe(method(inputContext))]
        fn input_context(&self) -> *mut AnyObject { std::ptr::null_mut() }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) { super::key_down(event); }

        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: &NSEvent) { super::key_up(event); }

        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &NSEvent) { super::flags_changed(event); }
    }
);

impl EditorView {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe {
            msg_send![mtm.alloc::<Self>(), initWithFrame: NSRect::new(NSPoint::ZERO, NSSize::new(400.0, 300.0))]
        }
    }
}
