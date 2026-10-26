//! 句末译文朗读配置；默认关闭，服务仅允许本机 HTTP。
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpeechConfig {
    pub enabled: bool,

    pub base_url: String,

    pub voice: String,

    pub speed: f64,

    pub timeout_ms: u64,
}
impl Default for SpeechConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "http://127.0.0.1:8766".into(),
            voice: "Vivian".into(),
            speed: 1.0,
            timeout_ms: 3000,
        }
    }
}
