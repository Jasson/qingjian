//! 父类组句回调的有限元数据；不采集返回对象的内容、地址或长度。

use crate::imk::draft_diagnostics::snapshot::Snapshot;

pub(in crate::imk::draft_diagnostics) struct Record {
    pub id: u64,

    pub session: u64,

    pub name: &'static str,

    pub parent_event_id: Option<u64>,

    pub parent_session: Option<u64>,

    pub before: Option<Snapshot>,
}
