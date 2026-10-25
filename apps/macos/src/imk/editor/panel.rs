//! 临时切换输入法应用激活策略后获得键盘焦点的独立编辑面板。

use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSEventType, NSPanel};

define_class!(
    // SAFETY: NSPanel 允许子类化；不保存额外 Objective-C 状态。
    #[unsafe(super(NSPanel))]
    #[thread_kind = MainThreadOnly]
    pub(super) struct EditorPanel;

    impl EditorPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool { true }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool { false }

        // AppKit 对 Command 组合的 keyUp 不总转给 NSView；自有面板保持完整按住周期。
        #[unsafe(method(sendEvent:))]
        fn send_event(&self, event: &NSEvent) {
            if super::CURRENT_PANEL.get() != self as *const Self as usize { return; }
            match event.r#type() {
                NSEventType::KeyDown => super::key_down(event),
                NSEventType::KeyUp => super::key_up(event),
                NSEventType::FlagsChanged => super::flags_changed(event),
                _ => unsafe { let _: () = msg_send![super(self), sendEvent: event]; },
            }
        }

        #[unsafe(method(becomeKeyWindow))]
        fn become_key(&self) {
            unsafe { let _: () = msg_send![super(self), becomeKeyWindow]; }
            super::panel_focus_acquired(self as *const Self as usize);
        }

        #[unsafe(method(resignKeyWindow))]
        fn resign_key(&self) {
            unsafe { let _: () = msg_send![super(self), resignKeyWindow]; }
            super::focus_lost(self as *const Self as usize);
        }
    }
);
