//! 只处理受控提交文本；恢复历史建立基线，不自动重读。
use crate::live::sentence::{is_closer, is_terminal, period_ends};
#[derive(Default)]
pub(super) struct Planner {
    text: String,

    consumed: usize,
}
impl Planner {
    pub fn baseline(&mut self, text: &str) {
        self.text = text.into();
        self.consumed = 0;
        // 建立基线只跳过历史完成句，用户继续编辑的尾句须保留整句。
        let _ = self.update(text);
    }

    pub fn discard_prefix(&mut self, bytes: usize) {
        if bytes >= self.text.len() {
            self.text.clear();
            self.consumed = 0;
        } else if self.text.is_char_boundary(bytes) {
            self.text.drain(..bytes);
            self.consumed = self.consumed.saturating_sub(bytes);
        }
    }

    pub fn update(&mut self, text: &str) -> (bool, Vec<String>) {
        let mut invalidated = false;
        if !text.starts_with(&self.text) {
            let common = self
                .text
                .chars()
                .zip(text.chars())
                .take_while(|(a, b)| a == b)
                .map(|(ch, _)| ch.len_utf8())
                .sum::<usize>();
            if common < self.consumed {
                invalidated = true;
                let chars: Vec<_> = text.char_indices().collect();
                self.consumed = chars
                    .iter()
                    .enumerate()
                    .filter(|(i, (offset, ch))| {
                        offset + ch.len_utf8() <= common
                            && is_terminal(*ch)
                            && (*ch != '.' || period_ends(text, &chars, *i))
                    })
                    .map(|(_, (offset, ch))| offset + ch.len_utf8())
                    .next_back()
                    .unwrap_or(0);
            }
        }
        self.text = text.into();
        let chars: Vec<_> = text.char_indices().collect();
        let mut sentences = Vec::new();
        let mut start = self.consumed;
        for (i, &(offset, ch)) in chars.iter().enumerate() {
            if offset < start || !is_terminal(ch) || (ch == '.' && !period_ends(text, &chars, i)) {
                continue;
            }
            let mut end = offset + ch.len_utf8();
            for &(next_offset, next) in &chars[i + 1..] {
                if is_closer(next) || is_terminal(next) {
                    end = next_offset + next.len_utf8();
                } else {
                    break;
                }
            }
            let sentence = text[start..end].trim();
            if sentence.chars().any(char::is_alphabetic) {
                sentences.push(sentence.into());
            }
            start = end;
        }
        self.consumed = start;
        (invalidated, sentences)
    }
}
#[cfg(test)]
mod tests {
    use super::Planner;
    #[test]
    fn confirmed_multi_sentence_no_replay() {
        let mut p = Planner::default();
        assert_eq!(p.update("你好。世界！还没完").1, ["你好。", "世界！"]);
        assert!(p.update("你好。世界！还没完").1.is_empty());
        assert_eq!(p.update("你好。世界！还没完？").1, ["还没完？"]);
    }
    #[test]
    fn baseline_and_edit_cancel_without_reading_history() {
        let mut p = Planner::default();
        p.baseline("历史。");
        assert_eq!(p.update("历史。新句。").1, ["新句。"]);
        assert!(p.update("历史。新句").0);
        assert_eq!(p.update("历史。新句！").1, ["新句！"]);
    }
    #[test]
    fn unfinished_backspace_preserves_entire_sentence() {
        let mut p = Planner::default();
        p.update("你好worl");
        assert!(!p.update("你好wor").0);
        assert_eq!(p.update("你好world。").1, ["你好world。"]);
        assert!(p.update("你好world。？！”").1.is_empty());
    }
    #[test]
    fn tracker_history_trim_does_not_replay_consumed_sentences() {
        let mut tracker = crate::live::SentenceTracker::new();
        let mut planner = Planner::default();
        let history = "旧句。".repeat(3500);
        tracker.commit(&history);
        planner.update(tracker.committed());
        let next = "新句。".repeat(300);
        let before = tracker.committed().len();
        tracker.commit(&next);
        let removed = before + next.len() - tracker.committed().len();
        assert!(removed > 0);
        planner.discard_prefix(removed);
        let (invalidated, sentences) = planner.update(tracker.committed());
        assert!(!invalidated);
        assert_eq!(sentences, vec!["新句。"; 300]);
    }
    #[test]
    fn commas_decimal_abbreviation_are_not_sentences() {
        let mut p = Planner::default();
        assert!(p.update("Dr. Smith 3.14，继续；").1.is_empty());
        assert_eq!(
            p.update("Dr. Smith 3.14，继续；好。").1,
            ["Dr. Smith 3.14，继续；好。"]
        );
    }
}
