//! 选中诊断记录后才采集的只读 Host 元数据，不包含原文或译文。

use crate::host::Host;

pub(super) struct Metadata {
    pub live_enabled: bool,

    pub draft_enabled: bool,

    pub application_known: bool,

    pub terminal_application: bool,

    pub pinyin_chars: usize,

    pub draft_chars: usize,

    pub epoch: u64,
}

impl Metadata {
    pub fn capture(host: &Host) -> Self {
        let config = host.settings.config();
        Self {
            live_enabled: config.live_translation.enabled,
            draft_enabled: config.live_translation.terminal_draft,
            application_known: host.engine.application().is_some(),
            terminal_application: host.engine.application() == Some("com.apple.Terminal"),
            pinyin_chars: host.engine.composition().text().chars().count(),
            draft_chars: host.live.draft.source().chars().count(),
            epoch: host.live.epoch,
        }
    }
}
