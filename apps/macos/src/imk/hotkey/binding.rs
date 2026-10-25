//! 快捷键的 ABC 物理键码与 Carbon 修饰位；owner 区分不同 IMK 输入会话。

use qingjian_platform::KeyCombo;
use qingjian_predict::live::LiveTranslationConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Binding {
    pub combo: KeyCombo,

    pub owner: usize,
}

impl Binding {
    pub fn for_session(
        combo: Option<KeyCombo>,
        owner: Option<usize>,
        application: Option<&str>,
        config: &LiveTranslationConfig,
    ) -> Option<Self> {
        if !crate::imk::draft_policy::allowed(config, application, false) {
            return None;
        }
        Some(Self {
            combo: combo?,
            owner: owner?,
        })
        .filter(|binding| binding.owner != 0 && binding.valid())
    }

    pub fn key_code(self) -> Option<u32> {
        Some(match self.combo.key.to_ascii_lowercase() {
            'a' => 0,
            'b' => 11,
            'c' => 8,
            'd' => 2,
            'e' => 14,
            'f' => 3,
            'g' => 5,
            'h' => 4,
            'i' => 34,
            'j' => 38,
            'k' => 40,
            'l' => 37,
            'm' => 46,
            'n' => 45,
            'o' => 31,
            'p' => 35,
            'q' => 12,
            'r' => 15,
            's' => 1,
            't' => 17,
            'u' => 32,
            'v' => 9,
            'w' => 13,
            'x' => 7,
            'y' => 16,
            'z' => 6,
            _ => return None,
        })
    }

    pub fn modifiers(self) -> u32 {
        let mods = self.combo.modifiers;
        u32::from(mods.command) << 8
            | u32::from(mods.shift) << 9
            | u32::from(mods.option) << 11
            | u32::from(mods.control) << 12
    }

    pub fn valid(self) -> bool {
        self.key_code().is_some() && !self.combo.modifiers.is_empty()
    }
}
