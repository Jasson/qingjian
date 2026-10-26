//! 终端草稿：确认前只编辑受控原文，预览不重复计入正文，也不按标点截句。

#[derive(Debug, Default)]
pub struct TerminalDraft {
    committed: String,

    preview: String,

    revision: u64,
}

impl TerminalDraft {
    pub fn commit(&mut self, text: &str) -> bool {
        let Some(text) = normalize_terminal_output(text) else {
            return false;
        };
        self.preview.clear();
        self.committed.push_str(&text);
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn preview(&mut self, text: &str) {
        let text = normalize_terminal_output(text).unwrap_or_default();
        if self.preview != text {
            self.preview = text;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    pub fn backspace(&mut self) {
        self.preview.clear();
        self.committed.pop();
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn reset(&mut self) {
        self.committed.clear();
        self.preview.clear();
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn source(&self) -> String {
        format!("{}{}", self.committed, self.preview)
    }

    pub fn committed(&self) -> &str {
        &self.committed
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// 只能确认正文，不能把尚未选定的候选、旧请求或不同原文的译文输出。
    pub fn translation_output(
        &self,
        receipt_source: &str,
        translation: &str,
        latest: bool,
    ) -> Option<String> {
        if !latest
            || !self.preview.is_empty()
            || self.committed.trim().is_empty()
            || receipt_source != self.committed
        {
            return None;
        }
        normalize_terminal_output(translation).filter(|text| !text.trim().is_empty())
    }

    pub fn original_output(&self) -> Option<String> {
        if !self.preview.is_empty() || self.committed.trim().is_empty() {
            return None;
        }
        normalize_terminal_output(&self.committed)
    }

    /// 输出已确认正文与当前原始按键；高亮预览不能冒充用户尚未确认的字。
    pub fn original_output_with_pending(&self, raw_keys: &str) -> Option<String> {
        normalize_terminal_output(&format!("{}{}", self.committed, raw_keys))
            .filter(|text| !text.trim().is_empty())
    }
}

/// 终端输出只允许单行；CRLF 作为一个换行处理，其余控制字符直接拒绝。
pub fn normalize_terminal_output(text: &str) -> Option<String> {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                output.push(' ');
            }
            '\n' | '\u{2028}' | '\u{2029}' => output.push(' '),
            ch if ch.is_control() => return None,
            ch => output.push(ch),
        }
    }
    Some(output)
}

#[cfg(test)]
mod tests;
