//! 独立草稿与本机翻译状态，不借用普通 Host.live 的句子、存档或窗口。

use super::pending::Pending;
use super::target::Target;
use super::window::Window;
use crate::imk::mode_switch::ModeSwitch;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_foundation::{NSRect, NSTimer};
use qingjian_core::EngineSession;
use qingjian_platform::ShortcutConfig;
use qingjian_predict::live::{FocusedDraftModel, LiveTranslator};

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub(super) struct Runtime {
    pub window: std::rc::Rc<Window>,

    pub id: u64,

    pub origin_pid: i32,

    pub held_keys: super::keys::HeldKeys,

    pub cancel_on_release: bool,

    pub target: Option<Target>,

    pub session: EngineSession,

    pub model: FocusedDraftModel,

    pub translator: Option<LiveTranslator>,

    pub mode: ModeSwitch,

    pub shortcut: ShortcutConfig,

    pub config: qingjian_platform::Config,

    pub reconfigure_input: bool,

    pub anchor: NSRect,

    pub status: String,

    pub pending: Option<Pending>,

    pub writing: bool,

    pub candidate_start: usize,

    pub submitted_revision: Option<u64>,

    pub timer: Retained<NSTimer>,
}

impl Runtime {
    pub fn new(
        mtm: MainThreadMarker,
        target: Option<Target>,
        origin_pid: i32,
        anchor: NSRect,
        config: &qingjian_platform::Config,
    ) -> Self {
        let mut mode = crate::imk::modifiers::editor_mode();
        mode.configure(config.general.english_mode, config.shortcut.switch_mode);
        let translator = LiveTranslator::new(config.live_translation.clone());
        let status = translator
            .as_ref()
            .err()
            .cloned()
            .unwrap_or_else(|| "草稿窗已打开，原文暂留在这里".to_owned());
        Self {
            window: std::rc::Rc::new(Window::new(mtm)),
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            held_keys: Default::default(),
            cancel_on_release: false,
            target,
            origin_pid,
            session: EngineSession::default(),
            model: FocusedDraftModel::new(),
            translator: translator.ok(),
            mode,
            shortcut: config.shortcut,
            config: config.clone(),
            reconfigure_input: false,
            anchor,
            status,
            pending: None,
            writing: false,
            candidate_start: 0,
            submitted_revision: None,
            timer: super::ticker::start(mtm),
        }
    }

    pub fn submit(&mut self) {
        let revision = self.model.revision();
        if self.submitted_revision == Some(revision) {
            return;
        }
        self.submitted_revision = Some(revision);
        if let Some(translator) = &mut self.translator {
            let id = translator.submit(self.model.source());
            self.model.submitted(id);
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.timer.invalidate();
        self.window.hide();
    }
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
}

impl Runtime {
    /// 失焦期间可能收不到松键；仅核验本窗口已经按住的键，不监听其他输入。
    pub fn prune_released_keys(&mut self) {
        self.held_keys
            .retain_pressed(|key| unsafe { CGEventSourceKeyState(0, key) });
    }
}
