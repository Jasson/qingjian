//! 草稿动作只接受回车、Esc 或修饰键加英文字母，不扩展通用组合键。

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::KeyCombo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum DraftShortcut {
    Enter,

    Escape,

    Combo(KeyCombo),
}

impl DraftShortcut {
    pub fn key_string(self) -> String {
        match self {
            Self::Enter => "enter".to_owned(),
            Self::Escape => "escape".to_owned(),
            Self::Combo(combo) => combo.key_string(),
        }
    }
}

impl FromStr for DraftShortcut {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.trim().to_ascii_lowercase().as_str() {
            "enter" | "return" | "numpadenter" | "numpad_enter" => Ok(Self::Enter),
            "escape" | "esc" => Ok(Self::Escape),
            text => {
                let combo: KeyCombo = text.parse()?;
                if !combo.key.is_ascii_alphabetic() || combo.modifiers.is_empty() {
                    return Err(
                        "draft shortcut requires Enter, Escape or modifiers plus an ASCII letter"
                            .to_owned(),
                    );
                }
                Ok(Self::Combo(combo))
            }
        }
    }
}

impl TryFrom<String> for DraftShortcut {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<DraftShortcut> for String {
    fn from(value: DraftShortcut) -> Self {
        value.key_string()
    }
}
