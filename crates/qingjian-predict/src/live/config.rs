//! 实时翻译的独立配置，不复用云联想凭据。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveTranslationConfig {
    pub enabled: bool,

    /// 仅在手动确认且选区完整匹配已有译文源文时允许替换。
    pub allow_replace: bool,

    /// 仅在系统 Terminal 中先保留原文草稿，手动确认后输出译文。
    pub terminal_draft: bool,

    pub base_url: String,

    pub model: String,

    pub debounce_ms: u64,

    pub timeout_ms: u64,
}

impl Default for LiveTranslationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allow_replace: false,
            terminal_draft: false,
            base_url: "http://localhost:11434".into(),
            model: "maternion/hy-mt2:1.8b".into(),
            debounce_ms: 180,
            timeout_ms: 5000,
        }
    }
}
