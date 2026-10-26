//! 实时翻译的独立配置，不复用云联想凭据。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveTranslationConfig {
    pub enabled: bool,

    /// 仅在手动确认且选区完整匹配已有译文源文时允许替换。
    pub allow_replace: bool,

    /// 在允许的应用中先保留原文草稿，手动确认后再输出。
    pub draft_enabled: bool,

    /// 不启用草稿的应用标识，由平台提供当前应用身份。
    pub draft_excluded_apps: Vec<String>,

    /// 已弃用：仅兼容旧配置，不再决定草稿运行策略。
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
            draft_enabled: true,
            draft_excluded_apps: Vec::new(),
            terminal_draft: false,
            base_url: "http://localhost:11434".into(),
            model: "maternion/hy-mt2:1.8b".into(),
            debounce_ms: 180,
            timeout_ms: 5000,
        }
    }
}

impl LiveTranslationConfig {
    /// 平台仍须独立排除安全输入环境，未知应用不启用草稿。
    pub fn draft_allowed_in(&self, application: Option<&str>) -> bool {
        self.enabled
            && self.draft_enabled
            && application.is_some_and(|application| {
                !application.trim().is_empty()
                    && !self
                        .draft_excluded_apps
                        .iter()
                        .any(|excluded| excluded == application)
            })
    }
}
