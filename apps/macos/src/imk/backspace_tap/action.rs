//! 草稿事件的编辑或显式动作；确认动作不按自动重复累计。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Backspace,

    InsertOriginal,

    CancelDraft,

    PasteDraft,

    PasteConflict,
}
