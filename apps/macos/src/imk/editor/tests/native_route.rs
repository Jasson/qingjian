//! 真实 ObjC 事件入口回放：不初始化 Host、不发送系统按键或客户端 IPC。

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2::{AnyThread, msg_send};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
use objc2_foundation::{NSObject, NSPoint, NSString};
use objc2_input_method_kit::IMKServer;

use super::scope::OwnerScope;
use crate::host;
use crate::imk::{controller::QingjianInputController, editor::owner::Owner};

#[test]
#[ignore = "本机手动真实 ObjC 路由探针；系统须接受 nil-server 指定初始化"]
fn actual_imk_entry_consumes_stale_y_while_opening_and_visible_then_restores_passthrough() {
    assert!(host::with(|_| ()).is_none());
    assert!(crate::imk::controller::active_controller_id().is_none());
    let controller: Option<Retained<QingjianInputController>> = unsafe {
        msg_send![QingjianInputController::alloc(),
            initWithServer: Option::<&IMKServer>::None,
            delegate: Option::<&AnyObject>::None,
            client: Option::<&AnyObject>::None]
    };
    let controller = controller.expect("系统拒绝 nil-server 初始化，探针不能运行");
    let text = NSString::from_str("y");
    let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        NSEventType::KeyDown, NSPoint::ZERO,
        NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
        0.0, 0, None, &text, &text, false, 16,
    ).expect("无法构造本地 NSEvent");
    let client = NSObject::new();
    let _scope = OwnerScope::enter(Owner::Idle);
    let dispatch = || -> bool {
        let handled: Bool =
            unsafe { msg_send![&*controller, handleEvent: &*event, client: &*client] };
        handled.as_bool()
    };
    assert!(!dispatch(), "对照：无活动 controller，普通路线应透传");
    let (_, origin_pid) =
        crate::imk::frontmost::identity().expect("无法只读核实前台应用，探针停止");
    for phase in [Owner::Opening(origin_pid), Owner::Visible(origin_pid)] {
        phase.set("test_native_route");
        assert!(dispatch(), "独立工作流须在旧客户端准备前消费陈旧Y事件");
        unsafe {
            let _: () = msg_send![&*controller, activateServer: Option::<&AnyObject>::None];
            let _: () = msg_send![&*controller, deactivateServer: Option::<&AnyObject>::None];
        }
        assert!(host::with(|_| ()).is_none());
        assert_eq!(Owner::current(), phase);
        assert!(dispatch(), "IMK 生命周期回调重入后不应放行陈旧输出键");
    }
    Owner::Idle.set("test_native_close");
    unsafe {
        let _: () = msg_send![&*controller, deactivateServer: Option::<&AnyObject>::None];
    }
    assert!(crate::imk::controller::active_controller_id().is_none());
    assert!(!dispatch(), "关闭后应恢复普通透传路线");
}
