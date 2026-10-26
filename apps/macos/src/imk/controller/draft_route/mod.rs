//! 草稿按键分流：内容存在时禁止透传命令，输出完成后的空态仍可正常操作终端。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DraftKeyRoute {
    StageInput,

    HoldDraft,

    PassApplication,
}

impl DraftKeyRoute {
    /// 仅自动草稿的真空态非语言首键透传；外层兜底共用返回后的路由。
    pub fn passes_empty_input(
        text: Option<&str>,
        draft_active: bool,
        source: &str,
        composing: bool,
        occupied: bool,
        question_mark: bool,
    ) -> bool {
        draft_active
            && qingjian_predict::live::passes_automatic_empty(
                text,
                source,
                composing,
                occupied,
                question_mark,
            )
    }

    pub fn pass_empty_input(
        &mut self,
        text: Option<&str>,
        draft_active: bool,
        source: &str,
        composing: bool,
        occupied: bool,
        question_mark: bool,
    ) -> bool {
        if !Self::passes_empty_input(
            text,
            draft_active,
            source,
            composing,
            occupied,
            question_mark,
        ) {
            return false;
        }
        *self = Self::PassApplication;
        true
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
            (true, false) => Self::PassApplication,
        }
    }

    pub fn consume_fallback(self, staged: bool, key_down: bool, handled: bool) -> bool {
        handled || (staged && key_down && self != Self::PassApplication)
    }
}

#[cfg(test)]
mod tests;
