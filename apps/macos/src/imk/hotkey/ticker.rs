//! 主线程轮询：核验注册生命周期并在 Host 释放后派发已绑定原文的热键意图。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::{NSObject, NSObjectProtocol};

define_class!(
    // SAFETY: NSObject 可直接子类化；只在主线程使用。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    pub(super) struct Ticker;

    impl Ticker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            if crate::imk::catch_panic("系统译文热键", super::poll).is_none() {
                super::disable();
            }
        }
    }

    unsafe impl NSObjectProtocol for Ticker {}
);

impl Ticker {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
