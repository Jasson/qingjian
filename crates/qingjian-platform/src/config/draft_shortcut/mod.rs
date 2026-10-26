//! 草稿动作的有限快捷键集合与可禁用的配置读写。

mod optional;
mod value;

pub(super) use optional::{deserialize, serialize};
pub use value::DraftShortcut;
