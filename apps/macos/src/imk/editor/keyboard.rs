//! 自有视图的按键翻译；不借客户端、不转发原始按键，不调用 interpretKeyEvents。

use super::runtime::Runtime;
use super::session::SessionLease;
use crate::host;
use objc2_app_kit::{NSEvent, NSEventModifierFlags};
use qingjian_platform::{DraftShortcut, KeyCombo, Modifiers};

fn modifiers(event: &NSEvent) -> Modifiers {
    let flags = event.modifierFlags();
    Modifiers {
        command: flags.contains(NSEventModifierFlags::Command),
        control: flags.contains(NSEventModifierFlags::Control),
        option: flags.contains(NSEventModifierFlags::Option),
        shift: flags.contains(NSEventModifierFlags::Shift),
    }
}

pub(super) fn matches(combo: Option<KeyCombo>, event: &NSEvent) -> bool {
    combo.is_some_and(|combo| {
        combo.modifiers == modifiers(event)
            && event.charactersIgnoringModifiers().is_some_and(|text| {
                text.to_string()
                    .eq_ignore_ascii_case(&combo.key.to_string())
            })
    })
}

pub(super) fn action_matches(shortcut: Option<DraftShortcut>, event: &NSEvent) -> bool {
    match shortcut {
        None => false,
        Some(DraftShortcut::Enter) => {
            matches!(event.keyCode(), 36 | 76) && modifiers(event).is_empty()
        }
        Some(DraftShortcut::Escape) => event.keyCode() == 53 && modifiers(event).is_empty(),
        Some(DraftShortcut::Combo(combo)) => matches(Some(combo), event),
    }
}

impl Runtime {
    pub fn key(&mut self, event: &NSEvent) -> Option<qingjian_render::ActionTarget> {
        use qingjian_render::ActionTarget;
        if self.writing || !self.window.owns_keyboard() {
            return None;
        }
        self.held_keys.press(event.keyCode());
        if self.cancel_on_release {
            return None;
        }
        if self.reconfigure_input {
            host::with(|h| {
                let mut engine = SessionLease::new(&mut h.engine, &mut self.session);
                self.model.commit_raw(&mut engine);
                self.model.refresh(&mut engine);
            });
            self.reconfigure_input = false;
            self.submit();
        }
        let switched = self.mode.key_down(
            event.keyCode(),
            event.modifierFlags().bits(),
            event.isARepeat(),
        );
        if switched {
            self.pending = None;
            host::with(|h| {
                let mut engine = SessionLease::new(&mut h.engine, &mut self.session);
                self.model.commit_raw(&mut engine);
            });
            self.mode.did_finish();
            self.submit();
            self.render();
            return None;
        }
        if !self.shortcut.draft_shortcuts_conflict()
            && action_matches(self.shortcut.cancel_draft, event)
        {
            return Some(ActionTarget::CancelDraft);
        }
        if !self.shortcut.draft_shortcuts_conflict()
            && action_matches(self.shortcut.insert_original, event)
        {
            return (!event.isARepeat()).then_some(ActionTarget::InsertOriginal);
        }
        if matches(self.shortcut.insert_live_translation, event) {
            return (!event.isARepeat()).then_some(ActionTarget::InsertTranslation);
        }
        if matches(self.shortcut.copy_live_translation, event) {
            return Some(ActionTarget::CopyTranslation);
        }
        let pressed = modifiers(event);
        if pressed.command || pressed.control || pressed.option {
            self.status = "草稿窗只接受文字与已配置的草稿快捷键".into();
            self.render();
            return None;
        }
        self.pending = None;
        let changed = host::with(|h| {
            let mut engine = SessionLease::new(&mut h.engine, &mut self.session);
            match event.keyCode() {
                51 => {
                    self.model.backspace(&mut engine);
                }
                123 => self.model.move_highlight(-1),
                124 => self.model.move_highlight(1),
                126 => self.model.move_highlight(-1),
                125 => self.model.move_highlight(1),
                48 => {
                    self.model.choose(&mut engine, self.model.highlighted());
                }
                _ => {
                    if let Some(text) = event.characters() {
                        if handle_digit(
                            &mut self.model,
                            &mut engine,
                            &text.to_string(),
                            pressed.is_empty(),
                        ) {
                            return true;
                        }
                        self.model.text(
                            &mut engine,
                            &text.to_string(),
                            self.mode.english(),
                            h.settings.config().general.punct_commits,
                        );
                    }
                }
            }
            true
        })
        .unwrap_or(false);
        if changed {
            self.status = "原文仅保存在草稿窗；确认后只填入文字，不发送消息".into();
            self.submit();
        }
        self.render();
        None
    }

    pub fn flags(&mut self, event: &NSEvent) {
        if self.writing || !self.window.owns_keyboard() {
            return;
        }
        if self
            .mode
            .flags(event.keyCode(), event.modifierFlags().bits())
        {
            self.pending = None;
            host::with(|h| {
                let mut engine = SessionLease::new(&mut h.engine, &mut self.session);
                self.model.commit_raw(&mut engine);
            });
            self.mode.did_finish();
            self.status = if self.mode.english() {
                "英文输入"
            } else {
                "中文输入"
            }
            .into();
            self.submit();
            self.render();
        }
    }
}

pub(super) fn digit_index(highlighted: usize, digit: usize, count: usize) -> Option<usize> {
    if !(1..=9).contains(&digit) {
        return None;
    }
    let index = highlighted / 9 * 9 + digit - 1;
    (index < count).then_some(index)
}

/// 数字只对应当前页；空槽也必须消费，不能落入模型的第一页数字选词。
pub(super) fn handle_digit(
    model: &mut qingjian_predict::live::FocusedDraftModel,
    engine: &mut qingjian_core::Engine,
    text: &str,
    unmodified: bool,
) -> bool {
    if !unmodified
        || model.raw_keys().is_empty()
        || engine.expression_mode()
        || engine.raw_mode()
        || engine.question_mode() && engine.unicode_entry()
    {
        return false;
    }
    let Some(digit) = text
        .parse::<usize>()
        .ok()
        .filter(|digit| (1..=9).contains(digit))
    else {
        return false;
    };
    if let Some(index) = digit_index(model.highlighted(), digit, model.candidates().len()) {
        model.choose(engine, index);
    }
    true
}
