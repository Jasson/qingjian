//! 草稿路由与外层透传兜底的真实决策回放。

use super::DraftKeyRoute;

#[test]
fn active_enter_and_control_commands_never_pass_but_empty_terminal_remains_usable() {
    for (key, control) in [
        (36, false),
        (76, false),
        (8, true),
        (123, false),
        (51, false),
    ] {
        let active = DraftKeyRoute::for_key(key, control, true);
        assert_eq!(active, DraftKeyRoute::HoldDraft);
        assert!(active.consume_fallback(true, true, false));
        let empty = DraftKeyRoute::for_key(key, control, false);
        assert_eq!(empty, DraftKeyRoute::PassApplication);
        assert!(!empty.consume_fallback(true, true, false));
    }
}

#[test]
fn printable_first_key_and_failed_dispatch_are_staged_but_normal_mode_is_unchanged() {
    for pending in [false, true] {
        for key in [0, 18, 49, 47] {
            let route = DraftKeyRoute::for_key(key, false, pending);
            assert_eq!(route, DraftKeyRoute::StageInput);
            assert!(route.consume_fallback(true, true, false));
            assert!(!route.consume_fallback(false, true, false));
            assert!(!route.consume_fallback(true, false, false));
            assert!(route.consume_fallback(false, true, true));
        }
    }
}

mod automatic;
mod restore;
