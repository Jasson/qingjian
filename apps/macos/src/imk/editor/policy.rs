//! 构造可交互窗口前准备激活策略，构造中异常时恢复原有后台策略。

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

pub(super) struct Policy {
    mtm: MainThreadMarker,

    previous: Option<NSApplicationActivationPolicy>,
}

impl Policy {
    pub(super) fn prepare(mtm: MainThreadMarker) -> Self {
        let app = NSApplication::sharedApplication(mtm);
        let previous = app.activationPolicy();
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        Self {
            mtm,
            previous: Some(previous),
        }
    }

    pub(super) fn transfer(mut self) -> NSApplicationActivationPolicy {
        self.previous.take().expect("激活策略尚未转移")
    }
}

impl Drop for Policy {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            NSApplication::sharedApplication(self.mtm).setActivationPolicy(previous);
        }
    }
}
