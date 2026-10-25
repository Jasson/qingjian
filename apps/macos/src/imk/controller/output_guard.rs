//! 确认输出的事件来源与输入凭据；系统回调与 IMK 共享最后一次消费门禁。

use crate::host::{Host, ObservationToken};
use crate::imk::{backspace_tap, hotkey};

#[derive(Clone, Copy)]
pub(super) enum OutputGuard<'a> {
    Carbon(u32, &'a ObservationToken),

    Tap(u64, u64, &'a ObservationToken),

    Snapshot(&'a ObservationToken),
}

impl OutputGuard<'_> {
    pub fn generation(self) -> u64 {
        match self {
            Self::Carbon(generation, _) => u64::from(generation),
            Self::Tap(generation, _, _) => generation,
            Self::Snapshot(_) => 0,
        }
    }

    pub fn valid(self, host: &Host) -> bool {
        if crate::imk::editor::session_owned() {
            return false;
        }
        let (registered, observation) = match self {
            Self::Carbon(generation, observation) => (
                hotkey::current(generation)
                    && hotkey::binding_combo(generation).is_some_and(|combo| {
                        host.settings.config().shortcut.insert_live_translation == Some(combo)
                    }),
                observation,
            ),
            Self::Tap(generation, serial, observation) => {
                (backspace_tap::current(generation, serial), observation)
            }
            Self::Snapshot(observation) => (true, observation),
        };
        registered && observation.matches(&host.live_observation_token())
    }
}
