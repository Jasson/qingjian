//! 空串禁用草稿动作，其余写法由 DraftShortcut 检查。

use serde::{Deserialize, Deserializer, Serializer};

use super::DraftShortcut;

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<DraftShortcut>, D::Error> {
    let text = String::deserialize(deserializer)?;
    if text.trim().is_empty() {
        Ok(None)
    } else {
        text.parse().map(Some).map_err(serde::de::Error::custom)
    }
}

pub(crate) fn serialize<S: Serializer>(
    value: &Option<DraftShortcut>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.map(DraftShortcut::key_string).unwrap_or_default())
}
