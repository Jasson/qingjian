//! 隔离的真实 ObjC ABI 探针；不启动 IMKServer/NSApplication，不发送系统按键。

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2::{AnyThread, msg_send};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
use objc2_foundation::{NSObject, NSPoint, NSString};
use objc2_input_method_kit::IMKServer;

use super::QingjianInputController;
use crate::host;

struct ActiveScope {
    controller: Retained<QingjianInputController>,

    previous: Option<Retained<QingjianInputController>>,
}

impl ActiveScope {
    fn enter(controller: Retained<QingjianInputController>) -> Self {
        let previous = super::click::active_controller();
        controller.set_active();
        Self {
            controller,
            previous,
        }
    }
}

impl Drop for ActiveScope {
    fn drop(&mut self) {
        self.controller.clear_active();
        if let Some(previous) = &self.previous {
            previous.set_active();
        }
    }
}

#[test]
#[ignore = "本机手动 ABI 探针；nil 指定初始化若不被系统接受则停止"]
fn objc_handle_event_returns_native_yes_for_host_unavailable_and_no_for_nil_client() {
    assert!(
        host::with(|_| ()).is_none(),
        "探针不能初始化或借用真实 Host"
    );
    assert!(
        super::click::active_controller().is_none(),
        "探针需要独立测试线程"
    );
    let controller: Option<Retained<QingjianInputController>> = unsafe {
        msg_send![QingjianInputController::alloc(),
            initWithServer: Option::<&IMKServer>::None,
            delegate: Option::<&AnyObject>::None,
            client: Option::<&AnyObject>::None]
    };
    let Some(controller) = controller else {
        panic!("ABI 探针不可用：系统拒绝 nil-server 指定初始化；未启动服务或应用");
    };
    let empty = NSString::from_str("");
    let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        NSEventType::KeyDown, NSPoint::ZERO, NSEventModifierFlags::empty(), 0.0, 0, None,
        &empty, &empty, false, 0,
    ).expect("系统无法构造本地 NSEvent；探针停止");
    let dummy = NSObject::new();
    let scope = ActiveScope::enter(controller);
    let handled: Bool =
        unsafe { msg_send![&*scope.controller, handleEvent: &*event, client: &*dummy] };
    assert!(handled.as_bool(), "真实 ObjC selector 未返回 YES");
    let missing_client: Bool = unsafe {
        msg_send![&*scope.controller, handleEvent: &*event, client: Option::<&AnyObject>::None]
    };
    assert!(!missing_client.as_bool(), "nil client 对照未返回 NO");
    assert!(host::with(|_| ()).is_none());
    drop(scope);
    assert!(super::click::active_controller().is_none());
    println!(
        "真实 ObjC ABI 探针：HostUnavailable/current-controller = YES；nil-client = NO；未发送系统按键或客户端 IPC"
    );
}
