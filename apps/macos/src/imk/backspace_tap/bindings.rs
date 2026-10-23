//! 本会话可捕获的动作配置；运行时拒绝手改配置中的冲突组合。

use qingjian_platform::ShortcutConfig;

use super::action::Action;
use super::key_binding::KeyBinding;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Bindings {
    original: Option<KeyBinding>,

    cancel: Option<KeyBinding>,
}

impl Bindings {
    pub fn for_config(shortcut: &ShortcutConfig) -> Self {
        if shortcut.draft_shortcuts_conflict() {
            return Self::default();
        }
        Self {
            original: shortcut.insert_original.and_then(KeyBinding::from_shortcut),
            cancel: shortcut.cancel_draft.and_then(KeyBinding::from_shortcut),
        }
    }

    pub fn action(self, key: i64, flags: u64) -> Option<Action> {
        if key == 51 && flags & ((1 << 18) | (1 << 19) | (1 << 20) | (1 << 23)) == 0 {
            Some(Action::Backspace)
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
