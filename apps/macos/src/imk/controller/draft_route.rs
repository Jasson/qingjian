//! 草稿按键分流：内容存在时禁止透传命令，输出完成后的空态仍可正常操作终端。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DraftKeyRoute {
    StageInput,

    HoldDraft,

    PassTerminal,
}

impl DraftKeyRoute {
    pub fn for_key(key: u16, command_or_control: bool, has_pending_text: bool) -> Self {
        let terminal_command = command_or_control
            || matches!(
                key,
                36 | 76 | 48 | 51 | 117 | 53 | 126 | 125 | 123 | 124 | 116 | 121 | 115 | 119
            );
        match (terminal_command, has_pending_text) {
            (false, _) => Self::StageInput,
            (true, true) => Self::HoldDraft,
            (true, false) => Self::PassTerminal,
        }
    }

    pub fn consume_fallback(self, staged: bool, key_down: bool, handled: bool) -> bool {
        handled || (staged && key_down && self != Self::PassTerminal)
    }
}

#[cfg(test)]
mod tests {
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
            assert_eq!(empty, DraftKeyRoute::PassTerminal);
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
}
