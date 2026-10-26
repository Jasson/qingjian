//! 重入栈帧；未被采样的回调也隔开外层写口计数。

use super::record::Record;

pub(super) struct Frame {
    pub ticket: u64,

    pub record: Option<Record>,
}
