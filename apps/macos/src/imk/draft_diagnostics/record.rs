//! 单个 IMK 回调的有限元数据；不包含字符、普通键码或文本长度。

use super::counts::WriteCounts;
use super::snapshot::Snapshot;

#[derive(Clone, Debug)]
pub(super) struct Record {
    pub id: u64,

    pub session: u64,

    pub kind: &'static str,

    pub before: Option<Snapshot>,

    pub current_before: bool,

    pub client_present: bool,

    pub prepared: &'static str,

    pub panicked: bool,

    pub writes: [WriteCounts; 4],
}
