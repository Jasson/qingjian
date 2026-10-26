//! 原生探针通过正式 NSApplication::run 启动，避免手动 finishLaunching 掩盖生命周期差异。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::NSApplication;
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};
use qingjian_predict::live::FocusedDraftModel;
use std::cell::RefCell;

thread_local! { static MODEL: RefCell<Option<FocusedDraftModel>> = const { RefCell::new(None) }; }

define_class!(
    // SAFETY: 只由主线程的本进程单次定时器触发，无外部 IPC 或系统按键。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    struct Runner;

    impl Runner {
        #[unsafe(method(runProbe:))]
        fn run_probe(&self, _timer: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            let model = MODEL.with(|model| model.borrow_mut().take()).expect("探针模型");
            let app = NSApplication::sharedApplication(mtm);
            std::process::exit(super::native_workflow(mtm, &app, model));
        }
    }
    unsafe impl NSObjectProtocol for Runner {}
);

pub(super) fn run(mtm: MainThreadMarker, app: &NSApplication, model: FocusedDraftModel) -> i32 {
    MODEL.with(|slot| *slot.borrow_mut() = Some(model));
    let target: Retained<Runner> = unsafe { msg_send![mtm.alloc::<Runner>(), init] };
    let _timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            0.05,
            &target,
            sel!(runProbe:),
            None,
            false,
        )
    };
    app.run();
    1
}
