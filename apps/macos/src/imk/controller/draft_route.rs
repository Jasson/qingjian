//! 草稿按键分流：内容存在时禁止透传命令，输出完成后的空态仍可正常操作终端。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DraftKeyRoute {
    StageInput,

    HoldDraft,

    PassTerminal,
}

impl DraftKeyRoute {
    /// 只按当前状态放行真空态单空格；空白草稿也属于已有内容，不能用 trim 判断。
    pub fn passes_empty_space(
        key: u16,
        text: Option<&str>,
        unmodified: bool,
        draft_active: bool,
        draft_source: &str,
        composing: bool,
        occupied: bool,
    ) -> bool {
        key == 49
            && text == Some(" ")
            && unmodified
            && draft_active
            && draft_source.is_empty()
            && !composing
            && !occupied
    }

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

    #[test]
    fn only_a_plain_single_space_in_an_unoccupied_truly_empty_terminal_draft_can_pass() {
        assert!(DraftKeyRoute::passes_empty_space(
            49,
            Some(" "),
            true,
            true,
            "",
            false,
            false
        ));
        for text in [None, Some(""), Some("  "), Some("\u{a0}"), Some("a")] {
            assert!(!DraftKeyRoute::passes_empty_space(
                49, text, true, true, "", false, false
            ));
        }
        assert!(!DraftKeyRoute::passes_empty_space(
            0,
            Some(" "),
            true,
            true,
            "",
            false,
            false
        ));
        assert!(!DraftKeyRoute::passes_empty_space(
            49,
            Some(" "),
            false,
            true,
            "",
            false,
            false
        ));
        assert!(!DraftKeyRoute::passes_empty_space(
            49,
            Some(" "),
            true,
            false,
            "",
            false,
            false
        ));
        assert!(!DraftKeyRoute::passes_empty_space(
            49,
            Some(" "),
            true,
            true,
            "",
            true,
            false
        ));
        assert!(!DraftKeyRoute::passes_empty_space(
            49,
            Some(" "),
            true,
            true,
            "",
            false,
            true
        ));
        for source in ["已有草稿", " ", "\n"] {
            assert!(!DraftKeyRoute::passes_empty_space(
                49,
                Some(" "),
                true,
                true,
                source,
                false,
                false
            ));
        }
    }

    #[test]
    fn late_space_passthrough_must_replace_the_initial_printable_route_for_outer_fallback() {
        let initial = DraftKeyRoute::for_key(49, false, false);
        assert_eq!(initial, DraftKeyRoute::StageInput);
        assert!(initial.consume_fallback(true, true, false));
        let current =
            if DraftKeyRoute::passes_empty_space(49, Some(" "), true, true, "", false, false) {
                DraftKeyRoute::PassTerminal
            } else {
                initial
            };
        assert!(!current.consume_fallback(true, true, false));
        // 已有草稿中的空格仍是正文输入；拼音中的空格也不能透传。
        let active = DraftKeyRoute::for_key(49, false, true);
        assert_eq!(active, DraftKeyRoute::StageInput);
        assert!(active.consume_fallback(true, true, false));
    }
}
