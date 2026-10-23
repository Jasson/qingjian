//! 可关闭的组合键：空串禁用，其他值沿用 KeyCombo 的写法。

use serde::{Deserialize, Deserializer, Serializer};

use super::KeyCombo;

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<KeyCombo>, D::Error> {
    let text = String::deserialize(deserializer)?;
    if text.trim().is_empty() {
        Ok(None)
    } else {
        text.parse().map(Some).map_err(serde::de::Error::custom)
    }
}

pub(super) fn serialize<S: Serializer>(
    combo: &Option<KeyCombo>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&combo.map(|key| key.key_string()).unwrap_or_default())
}

/// 实时翻译动作只占用字母组合，不能抢译词与删候选的修饰键加数字。
pub(super) fn deserialize_letter<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<KeyCombo>, D::Error> {
    let combo = deserialize(deserializer)?;
    match combo {
        Some(key) if !key.key.is_ascii_alphabetic() => Err(serde::de::Error::custom(
            "live translation shortcut requires modifiers and an ASCII letter",
        )),
        _ => Ok(combo),
    }
}
