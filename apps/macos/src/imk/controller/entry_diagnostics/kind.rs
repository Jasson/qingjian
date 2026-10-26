//! 入口诊断仅保留删除类别；普通按键不会把键码或文字带入记录。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    BackspaceKey,

    ForwardDeleteKey,

    BackspaceControl,

    DeleteControl,

    MissingEvent,

    MissingClient,

    NonDeleteKeyDown,

    Other,
}

impl Kind {
    pub fn classify(
        event: bool,
        client: bool,
        key_down: bool,
        key: Option<u16>,
        single: Option<u16>,
    ) -> Self {
        if !event {
            return Self::MissingEvent;
        }
        if !client {
            return Self::MissingClient;
        }
        if !key_down {
            return Self::Other;
        }
        match key {
            Some(51) => Self::BackspaceKey,
            Some(117) => Self::ForwardDeleteKey,
            _ => match single {
                Some(8) => Self::BackspaceControl,
                Some(127) => Self::DeleteControl,
                _ => Self::NonDeleteKeyDown,
            },
        }
    }

    pub fn key_code(self) -> Option<u16> {
        match self {
            Self::BackspaceKey => Some(51),
            Self::ForwardDeleteKey => Some(117),
            _ => None,
        }
    }

    pub fn special_slot(self) -> Option<usize> {
        match self {
            Self::BackspaceKey => Some(0),
            Self::ForwardDeleteKey => Some(1),
            Self::BackspaceControl => Some(2),
            Self::DeleteControl => Some(3),
            _ => None,
        }
    }
}
