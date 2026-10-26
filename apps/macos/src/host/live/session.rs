//! 每个激活输入会话的本地翻译状态；与选区翻译、云联想独立。

use objc2::MainThreadMarker;
use qingjian_platform::{CandidateRenderer, LayoutMode};
use qingjian_predict::live::context::{ContextGuard, ContextSnapshot};
use qingjian_predict::live::{
    DraftArchive, LiveTranslationConfig, LiveTranslator, SentenceTracker, TerminalDraft,
};

use super::display::DisplayState;
use super::monitor::LiveMonitor;
use super::raw_policy::RawPolicy;
use crate::candidates::CandidateWindow;

pub struct LiveSession {
    pub window: CandidateWindow,

    pub tracker: SentenceTracker,

    pub draft: TerminalDraft,

    pub archive: DraftArchive,

    pub raw_generation: u64,

    pub context: ContextGuard,

    pub translator: Option<LiveTranslator>,

    pub monitor: LiveMonitor,

    pub source: String,

    pub automatic: qingjian_predict::live::AutomaticSource,

    pub request_id: Option<u64>,

    pub paused: bool,

    pub last_context: Option<ContextSnapshot>,

    pub action_notice: Option<String>,

    pub system_hotkey_notice: Option<String>,

    pub backspace_tap_notice: Option<String>,

    pub(super) display: DisplayState,

    pub unavailable: Option<String>,

    pub epoch: u64,

    applied: Option<LiveTranslationConfig>,

    raw_policy: Option<RawPolicy>,
}

impl LiveSession {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let window = CandidateWindow::new(mtm);
        window.set_layout(LayoutMode::Vertical);
        window.set_renderer(CandidateRenderer::Qingjian);
        window.set_click_handler(Box::new(crate::imk::click));
        Self {
            window,
            tracker: SentenceTracker::default(),
            draft: TerminalDraft::default(),
            archive: DraftArchive::default(),
            raw_generation: 0,
            context: ContextGuard::default(),
            translator: None,
            monitor: LiveMonitor::new(mtm),
            source: String::new(),
            automatic: Default::default(),
            request_id: None,
            paused: false,
            last_context: None,
            action_notice: None,
            system_hotkey_notice: None,
            backspace_tap_notice: None,
            display: DisplayState::default(),
            unavailable: None,
            epoch: 0,
            applied: None,
            raw_policy: None,
        }
    }

    pub fn active(&self) -> bool {
        self.applied.as_ref().is_some_and(|config| config.enabled)
            && !crate::imk::secure_input::enabled()
    }

    pub fn draft_configured_in(&self, application: Option<&str>) -> bool {
        self.applied
            .as_ref()
            .is_some_and(|config| crate::imk::draft_policy::allowed(config, application, false))
    }

    pub fn configuration_changed(&self, config: &LiveTranslationConfig, force: bool) -> bool {
        force || self.applied.as_ref() != Some(config)
    }

    pub fn update_raw_policy(&mut self, config: &qingjian_platform::GeneralConfig) -> bool {
        let policy = RawPolicy::for_config(config);
        let changed = self.raw_policy.is_some_and(|previous| previous != policy);
        self.raw_policy = Some(policy);
        if changed {
            self.raw_generation = self.raw_generation.wrapping_add(1);
            self.archive.clear();
        }
        changed
    }

    pub fn configure(&mut self, config: &LiveTranslationConfig, force: bool) -> bool {
        if !force && self.applied.as_ref() == Some(config) {
            return false;
        }
        self.reset("configuration_changed");
        if !config.enabled {
            self.archive.clear();
        }
        self.unavailable = None;
        self.translator = if config.enabled {
            match LiveTranslator::new(config.clone()) {
                Ok(translator) => Some(translator),
                Err(error) => {
                    tracing::warn!(%error, "本地实时翻译未启用");
                    self.unavailable = Some(error);
                    None
                }
            }
        } else {
            None
        };
        self.applied = Some(config.clone());
        true
    }

    pub fn pause(&mut self, reason: &'static str) {
        if !self.paused && (!self.source.is_empty() || !self.tracker.committed().is_empty()) {
            tracing::info!(target: "live_translation", reason, epoch = self.epoch, "实时翻译停止追踪，保留当前成功译文");
        }
        if let Some(translator) = &mut self.translator {
            translator.cancel();
        }
        self.monitor.stop();
        self.tracker.reset();
        self.draft.reset();
        self.context.reset();
        self.request_id = None;
        self.last_context = None;
        self.action_notice = None;
        self.display.pause();
        self.source = self
            .display
            .cached_receipt(self.epoch)
            .map_or_else(String::new, |receipt| receipt.source.clone());
        self.paused = true;
        if self.source.is_empty() {
            self.window.hide();
        }
    }

    pub fn resume(&mut self) {
        if super::editing::resume(&mut self.paused, &mut self.source) {
            self.automatic.finish(&self.source, false);
            self.action_notice = None;
        }
    }

    pub fn reset(&mut self, reason: &'static str) {
        if !self.source.is_empty() || !self.tracker.committed().is_empty() {
            tracing::info!(target: "live_translation", reason, epoch = self.epoch, history_units = self.tracker.committed().encode_utf16().count(), "实时翻译会话重置");
        }
        self.epoch = self.epoch.wrapping_add(1);
        self.tracker.reset();
        self.draft.reset();
        self.context.reset();
        if let Some(translator) = &mut self.translator {
            translator.cancel();
        }
        self.source.clear();
        self.automatic = Default::default();
        self.request_id = None;
        self.paused = false;
        self.last_context = None;
        self.action_notice = None;
        self.display.reset();
        self.monitor.stop();
        self.window.hide();
    }
}
