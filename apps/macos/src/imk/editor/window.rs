//! 草稿的自绘窗口宿主；只有 key window 与自有第一响应者同时成立才接受编辑。

use super::panel::EditorPanel;
use super::view::EditorView;
use crate::candidates::{CandidateView, Frame, Theme};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSColor,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use qingjian_platform::{CandidateRenderer, LayoutMode};

pub(super) struct Window {
    panel: Retained<EditorPanel>,

    root: Retained<EditorView>,

    view: Retained<CandidateView>,

    previous_policy: std::cell::Cell<Option<NSApplicationActivationPolicy>>,
}

impl Window {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let policy = super::policy::Policy::prepare(mtm);
        let root = EditorView::new(mtm);
        let view = CandidateView::new(mtm, Theme::system_default());
        view.set_renderer(CandidateRenderer::Qingjian);
        view.set_layout(LayoutMode::Vertical);
        view.set_click_handler(Box::new(super::clicked));
        root.addSubview(&view);
        let panel: Retained<EditorPanel> = unsafe {
            msg_send![mtm.alloc::<EditorPanel>(),
            initWithContentRect: NSRect::new(NSPoint::ZERO, NSSize::new(400.0, 300.0)),
            styleMask: NSWindowStyleMask::Borderless,
            backing: NSBackingStoreType::Buffered, defer: false]
        };
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(false);
        panel.setHidesOnDeactivate(false);
        panel.setBecomesKeyOnlyIfNeeded(false);
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        panel.setLevel(101);
        panel.setContentView(Some(&root));
        Self {
            panel,
            root,
            view,
            previous_policy: std::cell::Cell::new(Some(policy.transfer())),
        }
    }

    pub fn render(&self, frame: Frame, anchor: NSRect) {
        let bounds = self.view.set_frame(&frame);
        self.view.setFrame(NSRect::new(NSPoint::ZERO, bounds.size));
        self.root.setFrame(NSRect::new(NSPoint::ZERO, bounds.size));
        let location = if anchor == NSRect::ZERO {
            objc2_app_kit::NSEvent::mouseLocation()
        } else {
            anchor.origin
        };
        self.panel.setFrame_display(
            NSRect::new(
                NSPoint::new(location.x, (location.y - bounds.size.height - 8.0).max(0.0)),
                bounds.size,
            ),
            true,
        );
    }

    pub fn focus(&self, current: impl Fn() -> bool) -> bool {
        if !current() {
            return false;
        }
        let mtm = objc2::MainThreadMarker::from(&*self.panel);
        let app = NSApplication::sharedApplication(mtm);
        if self.previous_policy.get().is_none() {
            self.previous_policy.set(Some(app.activationPolicy()));
        }
        if app.activationPolicy() != NSApplicationActivationPolicy::Accessory
            && !app.setActivationPolicy(NSApplicationActivationPolicy::Accessory)
        {
            return false;
        }
        for _ in 0..2 {
            if !current() {
                return false;
            }
            #[allow(deprecated)]
            app.activateIgnoringOtherApps(true);
            self.panel.makeKeyAndOrderFront(None);
            self.panel.makeKeyWindow();
            if self.panel.isKeyWindow() {
                break;
            }
            // 激活策略变更先由 AppKit/WindowServer 完成，再重试真正的激活请求。
            objc2_foundation::NSRunLoop::mainRunLoop().runUntilDate(
                &objc2_foundation::NSDate::dateWithTimeIntervalSinceNow(0.05),
            );
        }
        if !current() {
            return false;
        }
        self.panel.makeKeyWindow();
        let accepted = self.panel.makeFirstResponder(Some(&*self.root));
        let focused = accepted && self.owns_keyboard();
        tracing::info!(target:"live_translation", marker="focused-owner-v1", focused, key_window=self.panel.isKeyWindow(), active=app.isActive(), policy=?app.activationPolicy(), "草稿窗焦点核验");
        focused
    }

    pub fn acquire_responder(&self) -> bool {
        self.owns_keyboard()
            || (self.panel.isKeyWindow()
                && self.panel.makeFirstResponder(Some(&*self.root))
                && self.owns_keyboard())
    }

    pub fn owns_keyboard(&self) -> bool {
        self.panel.isKeyWindow()
            && self.panel.firstResponder().is_some_and(|responder| {
                std::ptr::eq(
                    &*responder as *const _,
                    &*self.root as *const EditorView as *const _,
                )
            })
    }

    pub fn show_notice(&self) {
        self.panel.orderFrontRegardless();
    }

    pub fn identity(&self) -> usize {
        &*self.panel as *const EditorPanel as usize
    }

    pub fn is_visible(&self) -> bool {
        self.panel.isVisible()
    }

    pub fn hide(&self) {
        self.panel.orderOut(None);
        if let Some(policy) = self.previous_policy.take() {
            let mtm = objc2::MainThreadMarker::from(&*self.panel);
            NSApplication::sharedApplication(mtm).setActivationPolicy(policy);
        }
    }
}

impl Window {
    pub(super) fn smoke_contract(&self) -> bool {
        let nil_context: *mut objc2::runtime::AnyObject =
            unsafe { msg_send![&*self.root, inputContext] };
        let focused = self.owns_keyboard();
        println!(
            "面板属性：canKey={}，canMain={}，accepts={}，nilContext={}，keyWindow={}，focus={focused}，owned={}",
            self.panel.canBecomeKeyWindow(),
            self.panel.canBecomeMainWindow(),
            self.root.acceptsFirstResponder(),
            nil_context.is_null(),
            self.panel.isKeyWindow(),
            self.owns_keyboard()
        );
        self.panel.canBecomeKeyWindow()
            && !self.panel.canBecomeMainWindow()
            && self.root.acceptsFirstResponder()
            && nil_context.is_null()
            && focused
            && self.owns_keyboard()
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.hide();
    }
}
