//! 自动翻译首段资格的会话状态；保留已启动稿件，不把后来剩下的数字当作新稿。

use super::{automatic_translation_visible, starts_automatic_source};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AutomaticSource {
    started: bool,

    sentence_start: Option<usize>,
}

impl AutomaticSource {
    pub fn observe(&mut self, text: &str) -> bool {
        self.started |= starts_automatic_source(text);
        self.started
    }

    /// 普通输入依照已有 SentenceTracker 的句界重新判定，不改动其断句规则。
    pub fn observe_sentence(&mut self, source: &str, raw: &str, start: usize) -> bool {
        if self.sentence_start != Some(start) {
            self.started = false;
        }
        self.sentence_start = Some(start);
        self.observe(raw);
        self.observe(source)
    }

    pub fn finish(&mut self, source: &str, composing: bool) {
        if source.is_empty() && !composing {
            *self = Self::default();
        }
    }

    /// 显式恢复已有草稿或接受 Core 面板选择，不构成自动截取非语言首键。
    pub fn restore(&mut self) {
        self.started = true;
    }

    pub fn visible(&self, source: &str, draft: &str, composing: bool) -> bool {
        self.started && automatic_translation_visible(source, draft, composing)
    }
}
