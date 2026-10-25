//! 草稿窗的文字和候选操作复用 Core，完全不读写原应用。

use qingjian_core::{Engine, QUESTION_PREFIX, punctuation, shortcut};

use super::FocusedDraftModel;
use crate::live::normalize_terminal_output;

impl FocusedDraftModel {
    pub fn choose(&mut self, engine: &mut Engine, index: usize) -> bool {
        let Some(candidate) = self.candidates.get(index).cloned() else {
            return false;
        };
        let text = engine.commit(&candidate);
        let accepted = self.draft.commit(&text);
        self.highlighted = 0;
        self.refresh(engine);
        accepted
    }

    pub fn commit_raw(&mut self, engine: &mut Engine) -> bool {
        if engine.composition().is_empty() {
            return false;
        }
        let raw = engine.take_raw();
        let accepted = self.draft.commit(&raw);
        self.highlighted = 0;
        self.refresh(engine);
        accepted
    }

    pub fn backspace(&mut self, engine: &mut Engine) -> bool {
        let changed = if !engine.composition().is_empty() {
            engine.backspace()
        } else if !self.draft.committed().is_empty() {
            self.draft.backspace();
            true
        } else {
            false
        };
        self.highlighted = 0;
        self.refresh(engine);
        changed
    }

    pub fn text(
        &mut self,
        engine: &mut Engine,
        text: &str,
        english: bool,
        punct_commits: bool,
    ) -> bool {
        let Some(text) = normalize_terminal_output(text) else {
            return false;
        };
        if english {
            self.commit_raw(engine);
            engine.set_english_mode(true);
            for ch in text.chars() {
                engine.note_passthrough(ch);
            }
            let accepted = self.draft.commit(&text);
            self.refresh(engine);
            return accepted;
        }
        engine.set_english_mode(false);
        for ch in text.chars() {
            self.character(engine, ch, punct_commits);
        }
        true
    }

    fn character(&mut self, engine: &mut Engine, ch: char, punct_commits: bool) {
        let composing = !engine.composition().is_empty();
        let question = composing && engine.question_mode();
        let expression = composing && engine.expression_mode();
        let raw = composing && engine.raw_mode();
        let enters_mode = !composing
            && (engine.takes_mode_letter(ch)
                || ch == QUESTION_PREFIX && engine.takes_question_mark());
        let accepts = ch.is_ascii_lowercase()
            || enters_mode
            || ch.is_ascii_uppercase() && engine.shift_letter_compose()
            || composing && ch == '\''
            || composing && ch == ';' && engine.takes_semicolon()
            || expression && shortcut::is_expression_char(ch)
            || raw && ch.is_ascii_graphic()
            || question && engine.unicode_entry() && (ch.is_ascii_digit() || ch == '+')
            || composing
                && ch.is_ascii_punctuation()
                && !(punct_commits && punctuation::converts(ch));
        if accepts {
            engine.push(ch);
            self.highlighted = 0;
            self.refresh(engine);
            return;
        }
        if composing && ch.is_ascii_uppercase() {
            self.commit_raw(engine);
        } else if composing && ch.is_ascii_digit() {
            let index = ch
                .to_digit(10)
                .and_then(|index| index.checked_sub(1))
                .map(|index| index as usize);
            if index.is_some_and(|index| self.choose(engine, index)) {
                return;
            }
            engine.push(ch);
            self.refresh(engine);
            return;
        } else if composing {
            if !self.choose(engine, self.highlighted) {
                self.commit_raw(engine);
            }
            if ch == ' ' && !raw {
                return;
            }
        }
        let text = engine.punctuate(ch).map(str::to_owned).unwrap_or_else(|| {
            engine.note_passthrough(ch);
            ch.to_string()
        });
        self.draft.commit(&text);
        self.refresh(engine);
    }
}
