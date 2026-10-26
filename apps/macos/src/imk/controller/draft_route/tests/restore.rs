//! 菜单恢复生产路由分段执行，客户端阶段不能持有资格检查的 Host 借用。

use std::cell::{Cell, RefCell};

use crate::imk::controller::archive::run_restore_if_ready;
use crate::menubar::MenuAction;

#[test]
fn restore_menu_has_its_own_action_and_disabled_readiness_never_calls_the_client() {
    assert_eq!(MenuAction::RestoreCurrentDraft.tag(), 7);
    assert_eq!(
        MenuAction::from_tag(7),
        Some(MenuAction::RestoreCurrentDraft)
    );
    assert_ne!(MenuAction::OpenSymbolPanel.tag(), 7);
    let readiness_calls = Cell::new(0);
    let client_calls = Cell::new(0);
    assert!(!run_restore_if_ready(
        || {
            readiness_calls.set(readiness_calls.get() + 1);
            false
        },
        || client_calls.set(client_calls.get() + 1),
    ));
    assert_eq!(readiness_calls.get(), 1);
    assert_eq!(client_calls.get(), 0);
}

#[test]
fn real_menu_route_releases_prepare_borrow_before_client_callback_and_restores_only_once() {
    let host = RefCell::new(Vec::new());
    assert!(run_restore_if_ready(
        || {
            let mut borrowed = host.borrow_mut();
            borrowed.push("资格检查");
            true
        },
        || {
            let mut reentrant = host
                .try_borrow_mut()
                .expect("客户端 IPC 阶段仍持有 Host 借用");
            reentrant.push("客户端读取");
            drop(reentrant);
            host.borrow_mut().push("恢复内部草稿");
        },
    ));
    assert_eq!(&*host.borrow(), &["资格检查", "客户端读取", "恢复内部草稿"]);
}
