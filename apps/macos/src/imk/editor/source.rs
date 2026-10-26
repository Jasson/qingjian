//! 只有提示后的明确快捷键才是填入当前新目标的确认；普通打开按钮不具有此权限。

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenSource {
    Shortcut,

    Button,
}
