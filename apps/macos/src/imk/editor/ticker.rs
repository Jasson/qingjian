//! 独立草稿本地翻译轮询与释放焦点后的确认重试，不读取按键正文日志。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

define_class!(
    // SAFETY: NSObject 的主线程定时器目标，无额外状态。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    pub(super) struct Ticker;

    impl Ticker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) { super::tick(); }
    }
    unsafe impl NSObjectProtocol for Ticker {}
);

pub(super) fn start(mtm: MainThreadMarker) -> Retained<NSTimer> {
    let target: Retained<Ticker> = unsafe { msg_send![mtm.alloc::<Ticker>(), init] };
    unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            0.05,
            &target,
            sel!(tick:),
            None,
            true,
        )
    }
}
