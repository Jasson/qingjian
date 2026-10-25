//! 仅观察已有父类回调；super 调用不被捕获、替换或主动触发。

mod record;
pub(super) use record::Record;

use crate::host;
use crate::imk::secure_input;

pub(super) struct Callback {
    record: Option<Record>,
}

impl Callback {
    pub fn begin(name: &'static str) -> Self {
        let record = super::isolated(|| {
            let secure = secure_input::enabled();
            let before = (!secure).then(host::draft_diagnostic_snapshot).flatten();
            super::STATE.with(|state| state.try_borrow_mut().ok()?.callback(name, before, secure))
        })
        .flatten();
        Self { record }
    }

    pub fn finish(mut self, current: impl FnOnce() -> bool) {
        self.log(true, || Some(current()));
    }

    fn log(&mut self, completed: bool, current: impl FnOnce() -> Option<bool>) {
        let Some(record) = self.record.take() else {
            return;
        };
        let _ = super::isolated(|| {
            if secure_input::enabled() {
                return;
            }
            let valid = super::STATE.with(|state| {
                state
                    .try_borrow()
                    .ok()
                    .is_some_and(|state| state.callback_current(record.session))
            });
            if !valid {
                return;
            }
            let after = host::draft_diagnostic_snapshot();
            if after.is_some_and(|value| !value.wechat) {
                return;
            }
            tracing::info!(target: "live_translation", phase = "callback", callback = record.name,
                callback_id = record.id, session = record.session, parent_event_id = record.parent_event_id,
                parent_session = record.parent_session, completed, current_controller = current(),
                host_available = after.is_some(), staged_before = record.before.map(|value| value.staged),
                staged_after = after.map(|value| value.staged),
                pid = std::process::id(), diagnostic_revision = "wechat-draft-v2", "微信草稿诊断");
        });
    }
}

impl Drop for Callback {
    fn drop(&mut self) {
        self.log(false, || None);
    }
}
