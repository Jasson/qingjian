//! 配置动作到 ABC 物理键与 CoreGraphics 修饰位的映射。

use qingjian_platform::DraftShortcut;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct KeyBinding {
    key: i64,

    flags: u64,
}

impl KeyBinding {
    pub fn from_shortcut(shortcut: DraftShortcut) -> Option<Self> {
        let (key, flags) = match shortcut {
            DraftShortcut::Enter => (36, 0),
            DraftShortcut::Escape => (53, 0),
            DraftShortcut::Combo(combo) => {
                let key = match combo.key.to_ascii_lowercase() {
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
                };
                let mods = combo.modifiers;
                if mods.is_empty() {
                    return None;
                }
                let flags = u64::from(mods.shift) << 17
                    | u64::from(mods.control) << 18
                    | u64::from(mods.option) << 19
                    | u64::from(mods.command) << 20;
                (key, flags)
            }
        };
        Some(Self { key, flags })
    }

    pub fn matches(self, key: i64, flags: u64) -> bool {
        let key = if key == 76 { 36 } else { key };
        key == self.key && flags & ((0xf << 17) | (1 << 23)) == self.flags
    }
}
