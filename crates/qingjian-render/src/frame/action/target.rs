//! 翻译操作的语义目标，不占用候选序号。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionTarget {
    CopyTranslation,

    ReplaceOriginal,

    InsertTranslation,

    InsertOriginal,

    CancelDraft,

    RestoreDraft,

    OpenTranslationDraft,
}

impl ActionTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::CopyTranslation => "复制译文",
            Self::ReplaceOriginal => "替换选中内容",
            Self::InsertTranslation => "输出译文",
            Self::InsertOriginal => "输出原文",
            Self::CancelDraft => "取消草稿",
            Self::RestoreDraft => "恢复草稿",
            Self::OpenTranslationDraft => "打开草稿窗",
        }
    }
}
