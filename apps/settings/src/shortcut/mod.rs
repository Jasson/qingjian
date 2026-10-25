//! 录快捷键：哪几项能录、按下的键能不能用（缺修饰键、跟别的撞了），以及各平台给人看的写法。

use bamboo_slip::{KeyCode, KeyPress};
use qingjian_platform::{Config, DraftShortcut, KeyCombo, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shortcut {
    Translation,
    TranslationSecond,
    DeleteCandidate,
    TranslateSelection,
    Predict,
    CopyLiveTranslation,
    ReplaceLiveTranslation,
    InsertLiveTranslation,
    OpenTranslationDraft,
    InsertOriginal,
    CancelDraft,
    /// 辅码触发键（单个符号，不带修饰键）。
    AuxCodeKey,
}

impl Shortcut {
    #[cfg(test)]
    pub const ALL: [Self; 12] = [
        Self::Translation,
        Self::TranslationSecond,
        Self::DeleteCandidate,
        Self::TranslateSelection,
        Self::Predict,
        Self::CopyLiveTranslation,
        Self::ReplaceLiveTranslation,
        Self::InsertLiveTranslation,
        Self::OpenTranslationDraft,
        Self::InsertOriginal,
        Self::CancelDraft,
        Self::AuxCodeKey,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Translation => "上屏第一个译词",
            Self::TranslationSecond => "上屏第二个译词",
            Self::DeleteCandidate => "删除候选",
            Self::TranslateSelection => "翻译选中文字",
            Self::Predict => "按需 AI 联想",
            Self::CopyLiveTranslation => "复制实时译文",
            Self::ReplaceLiveTranslation => "用实时译文替换选中内容",
            Self::InsertLiveTranslation => "输出草稿译文",
            Self::OpenTranslationDraft => "打开翻译草稿窗",
            Self::InsertOriginal => "输出草稿原文",
            Self::CancelDraft => "取消整段草稿",
            Self::AuxCodeKey => "触发键",
        }
    }

    /// 配置文件里的分节与键。
    pub fn config_key(self) -> (&'static str, &'static str) {
        match self {
            Self::Translation => ("shortcut", "translation"),
            Self::TranslationSecond => ("shortcut", "translation_second"),
            Self::DeleteCandidate => ("shortcut", "delete_candidate"),
            Self::TranslateSelection => ("shortcut", "translate_selection"),
            Self::Predict => ("shortcut", "predict"),
            Self::CopyLiveTranslation => ("shortcut", "copy_live_translation"),
            Self::ReplaceLiveTranslation => ("shortcut", "replace_live_translation"),
            Self::InsertLiveTranslation => ("shortcut", "insert_live_translation"),
            Self::OpenTranslationDraft => ("shortcut", "open_translation_draft"),
            Self::InsertOriginal => ("shortcut", "insert_original"),
            Self::CancelDraft => ("shortcut", "cancel_draft"),
            Self::AuxCodeKey => ("general", "aux_code_key"),
        }
    }

    /// 录制时的提示。
    pub fn prompt(self) -> &'static str {
        match self {
            Self::Translation | Self::TranslationSecond | Self::DeleteCandidate => {
                "按住修饰键再按个数字"
            }
            Self::TranslateSelection | Self::Predict => "按下修饰键 + 字母",
            Self::CopyLiveTranslation
            | Self::ReplaceLiveTranslation
            | Self::OpenTranslationDraft
            | Self::InsertLiveTranslation => "按修饰键 + 字母，退格禁用",
            Self::InsertOriginal | Self::CancelDraft => "Enter / Esc / 修饰字母；⌫禁用",
            Self::AuxCodeKey => "按一个符号键",
        }
    }

    /// 现在的值，给人看的写法。
    pub fn current(self, config: &Config) -> String {
        let shortcut = &config.shortcut;
        let (first, second) = shortcut.translation_keys();
        match self {
            Self::Translation => digits(first),
            Self::TranslationSecond => digits(second),
            Self::DeleteCandidate => digits(shortcut.delete_keys()),
            Self::TranslateSelection => combo_label(shortcut.translate_selection),
            Self::Predict => combo_label(shortcut.predict),
            Self::CopyLiveTranslation => shortcut
                .copy_live_translation
                .map(combo_label)
                .unwrap_or_else(|| "未启用".to_owned()),
            Self::ReplaceLiveTranslation => shortcut
                .replace_live_translation
                .map(combo_label)
                .unwrap_or_else(|| "未启用".to_owned()),
            Self::InsertLiveTranslation => shortcut
                .insert_live_translation
                .map(combo_label)
                .unwrap_or_else(|| "未启用".to_owned()),
            Self::OpenTranslationDraft => shortcut
                .open_translation_draft
                .map(combo_label)
                .unwrap_or_else(|| "未启用".to_owned()),
            Self::InsertOriginal => draft_label(shortcut.insert_original),
            Self::CancelDraft => draft_label(shortcut.cancel_draft),
            Self::AuxCodeKey => config.general.aux_code_key().to_string(),
        }
    }

    /// 按下的键能用就给出要写进配置的值，不能用给出原因（录制接着等）。
    pub fn accept(self, press: &KeyPress, config: &Config) -> Result<String, String> {
        let modifiers = modifiers(press);
        if matches!(
            self,
            Self::CopyLiveTranslation
                | Self::ReplaceLiveTranslation
                | Self::OpenTranslationDraft
                | Self::InsertLiveTranslation
                | Self::InsertOriginal
                | Self::CancelDraft
        ) && press.key == KeyCode::Backspace
            && modifiers.is_empty()
        {
            return Ok(String::new());
        }
        match self {
            Self::InsertOriginal | Self::CancelDraft => {
                let value = match &press.key {
                    KeyCode::Enter if modifiers.is_empty() => DraftShortcut::Enter,
                    KeyCode::Escape if modifiers.is_empty() => DraftShortcut::Escape,
                    KeyCode::Character(text) if !modifiers.is_empty() => {
                        let key = single(text)
                            .filter(char::is_ascii_alphabetic)
                            .ok_or("要回车、Esc 或修饰键加一个英文字母")?;
                        DraftShortcut::Combo(KeyCombo {
                            modifiers,
                            key: key.to_ascii_lowercase(),
                        })
                    }
                    _ => return Err("要回车、Esc 或修饰键加一个英文字母".to_owned()),
                };
                action_value(self, value, config)
            }
            Self::AuxCodeKey => {
                let key = match &press.key {
                    KeyCode::Character(text) if modifiers.is_empty() => single(text),
                    _ => None,
                }
                .ok_or("要不带修饰键的单个符号键")?;
                if qingjian_core::is_valid_aux_code_key(key, config.general.page_keys()) {
                    Ok(key.to_string())
                } else {
                    Err(format!(
                        "「{key}」不行：字母、数字、单引号与翻页键都不能当触发键"
                    ))
                }
            }
            Self::TranslateSelection
            | Self::Predict
            | Self::CopyLiveTranslation
            | Self::ReplaceLiveTranslation
            | Self::OpenTranslationDraft
            | Self::InsertLiveTranslation => {
                let key = match &press.key {
                    KeyCode::Character(text) => single(text).filter(|key| {
                        if matches!(
                            self,
                            Self::CopyLiveTranslation
                                | Self::ReplaceLiveTranslation
                                | Self::OpenTranslationDraft
                                | Self::InsertLiveTranslation
                        ) {
                            key.is_ascii_alphabetic()
                        } else {
                            key.is_ascii_alphanumeric() || (self == Self::Predict && *key == '.')
                        }
                    }),
                    _ => None,
                }
                .ok_or(
                    if matches!(
                        self,
                        Self::CopyLiveTranslation
                            | Self::ReplaceLiveTranslation
                            | Self::OpenTranslationDraft
                            | Self::InsertLiveTranslation
                    ) {
                        "要修饰键加一个字母"
                    } else {
                        "要修饰键加一个字母或数字"
                    },
                )?;
                if modifiers.is_empty() {
                    return Err("要带修饰键".to_owned());
                }
                let combo = KeyCombo {
                    modifiers,
                    key: key.to_ascii_lowercase(),
                };
                action_value(self, DraftShortcut::Combo(combo), config)
            }
            Self::Translation | Self::TranslationSecond | Self::DeleteCandidate => {
                if modifiers.is_empty() {
                    return Err("要按住修饰键".to_owned());
                }
                let shortcut = &config.shortcut;
                let (first, second) = shortcut.translation_keys();
                let others = [
                    (Self::Translation, first),
                    (Self::TranslationSecond, second),
                    (Self::DeleteCandidate, shortcut.delete_keys()),
                ];
                if let Some((other, _)) = others
                    .iter()
                    .find(|(other, keys)| *other != self && *keys == modifiers)
                {
                    return Err(format!("跟「{}」重复了", other.title()));
                }
                Ok(modifiers.key())
            }
        }
    }
}

fn action_value(action: Shortcut, value: DraftShortcut, config: &Config) -> Result<String, String> {
    let keys = &config.shortcut;
    let combos = [
        (
            Shortcut::TranslateSelection,
            Some(DraftShortcut::Combo(keys.translate_selection)),
        ),
        (Shortcut::Predict, Some(DraftShortcut::Combo(keys.predict))),
        (
            Shortcut::CopyLiveTranslation,
            keys.copy_live_translation.map(DraftShortcut::Combo),
        ),
        (
            Shortcut::ReplaceLiveTranslation,
            keys.replace_live_translation.map(DraftShortcut::Combo),
        ),
        (
            Shortcut::InsertLiveTranslation,
            keys.insert_live_translation.map(DraftShortcut::Combo),
        ),
        (Shortcut::InsertOriginal, keys.insert_original),
        (Shortcut::CancelDraft, keys.cancel_draft),
    ];
    let enhanced = |shortcut: Shortcut| {
        matches!(
            shortcut,
            Shortcut::OpenTranslationDraft
                | Shortcut::CopyLiveTranslation
                | Shortcut::ReplaceLiveTranslation
                | Shortcut::InsertLiveTranslation
                | Shortcut::InsertOriginal
                | Shortcut::CancelDraft
        )
    };
    if let Some((other, _)) = combos.iter().find(|(other, binding)| {
        *other != action
            && *binding == Some(value)
            && (enhanced(action)
                || (crate::support::Feature::LiveTranslation.here() && enhanced(*other)))
    }) {
        return Err(format!("跟「{}」重复了", other.title()));
    }
    Ok(value.key_string())
}

fn draft_label(value: Option<DraftShortcut>) -> String {
    match value {
        None => "未启用".to_owned(),
        Some(DraftShortcut::Enter) => "Enter".to_owned(),
        Some(DraftShortcut::Escape) => "Esc".to_owned(),
        Some(DraftShortcut::Combo(combo)) => combo_label(combo),
    }
}

fn single(text: &str) -> Option<char> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

/// 按键上按住的修饰键，换成配置的写法（Windows 上 command 记的是 Win 键）。
fn modifiers(press: &KeyPress) -> Modifiers {
    Modifiers {
        option: press.modifiers.alt,
        shift: press.modifiers.shift,
        control: press.modifiers.control,
        command: press.modifiers.logo,
    }
}

/// 修饰键的写法：macOS 用符号 ⌃⇧⌥⌘，其他平台写键名。
fn modifiers_label(modifiers: Modifiers) -> String {
    if cfg!(target_os = "macos") {
        return modifiers.label();
    }
    let mut names = Vec::new();
    if modifiers.control {
        names.push("Ctrl");
    }
    if modifiers.shift {
        names.push("Shift");
    }
    if modifiers.option {
        names.push("Alt");
    }
    if modifiers.command {
        names.push("Win");
    }
    names.join(" + ")
}

/// 修饰键配候选序号。
fn digits(modifiers: Modifiers) -> String {
    format!("{} + 序号", modifiers_label(modifiers))
}

fn combo_label(combo: KeyCombo) -> String {
    let key = combo.key.to_ascii_uppercase();
    if cfg!(target_os = "macos") {
        format!("{}{key}", modifiers_label(combo.modifiers))
    } else {
        format!("{} + {key}", modifiers_label(combo.modifiers))
    }
}

#[cfg(test)]
mod tests;
