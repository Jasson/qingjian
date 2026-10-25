//! 诊断写入口分类；普通写入包括精确范围替换。

#[derive(Clone, Copy)]
pub(crate) enum WriteKind {
    Marked,

    Ordinary,

    Confirmed,

    EmptyMarked,
}

impl WriteKind {
    pub(super) fn index(self) -> usize {
        match self {
            Self::Marked => 0,
            Self::Ordinary => 1,
            Self::Confirmed => 2,
            Self::EmptyMarked => 3,
        }
    }
}
