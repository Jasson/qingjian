//! 本会话可捕获的动作配置；运行时拒绝手改配置中的冲突组合。

use qingjian_platform::ShortcutConfig;

use super::action::Action;
use super::key_binding::KeyBinding;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Bindings {
    original: Option<KeyBinding>,

    cancel: Option<KeyBinding>,

    paste: Option<KeyBinding>,

    paste_conflict: bool,
}

impl Bindings {
    pub fn for_config(shortcut: &ShortcutConfig) -> Self {
        let mut bindings = Self {
            paste: shortcut.paste_draft.and_then(|combo| {
                KeyBinding::from_shortcut(qingjian_platform::DraftShortcut::Combo(combo))
            }),
            paste_conflict: shortcut.paste_draft_conflicts(),
            ..Self::default()
        };
        if !shortcut.draft_shortcuts_conflict() {
            bindings.original = shortcut.insert_original.and_then(KeyBinding::from_shortcut);
            bindings.cancel = shortcut.cancel_draft.and_then(KeyBinding::from_shortcut);
        }
        bindings
    }

    pub fn without_paste(mut self) -> Self {
        self.paste = None;
        self
    }

    pub fn action(self, key: i64, flags: u64) -> Option<Action> {
        if key == 51 && flags & ((1 << 18) | (1 << 19) | (1 << 20) | (1 << 23)) == 0 {
            Some(Action::Backspace)
        } else if self
            .paste
            .is_some_and(|binding| binding.matches(key, flags))
        {
            Some(if self.paste_conflict {
                Action::PasteConflict
            } else {
                Action::PasteDraft
            })
        } else if self
            .original
            .is_some_and(|binding| binding.matches(key, flags))
        {
            Some(Action::InsertOriginal)
        } else if self
            .cancel
            .is_some_and(|binding| binding.matches(key, flags))
        {
            Some(Action::CancelDraft)
        } else {
            None
        }
    }
}
