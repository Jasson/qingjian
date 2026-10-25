//! 同一自绘帧展示待选键、整段草稿、译文与候选，动作仍使用共享渲染语义。

use super::runtime::Runtime;
use crate::candidates::{Frame, Preedit, Row};
use qingjian_predict::live::FocusedDraftModel;
use qingjian_render::{Action, ActionTarget};

impl Runtime {
    pub fn render(&mut self) {
        let status = self.model.error().map_or_else(
            || self.status.clone(),
            |error| format!("{} · {error}", self.status),
        );
        let (frame, start) = build_frame(&self.model, &status, self.mode.english(), self.writing);
        self.candidate_start = start;
        self.window.render(frame, self.anchor);
    }
}

pub(super) fn build_frame(
    model: &FocusedDraftModel,
    status: &str,
    english: bool,
    writing: bool,
) -> (Frame, usize) {
    let mut rows = Vec::new();
    for line in qingjian_render::translation_lines(&format!("原文：{}", model.source()), 44, 3) {
        rows.push(Row {
            index: String::new(),
            text: line,
            annotation: Vec::new(),
            cloud: false,
        });
    }
    let translation = model.copy_output().unwrap_or("等待本机翻译…");
    for line in qingjian_render::translation_lines(&format!("译文：{translation}"), 44, 4) {
        rows.push(Row {
            index: String::new(),
            text: line,
            annotation: Vec::new(),
            cloud: false,
        });
    }
    for line in qingjian_render::translation_lines(status, 44, 3) {
        rows.push(Row {
            index: String::new(),
            text: line,
            annotation: Vec::new(),
            cloud: false,
        });
    }
    let candidate_start = rows.len();
    let page = model.highlighted() / 9 * 9;
    rows.extend(
        model
            .candidates()
            .iter()
            .enumerate()
            .skip(page)
            .take(9)
            .map(|(index, candidate)| Row::from_candidate(index - page, candidate)),
    );
    let selected = if model.candidates().is_empty() {
        usize::MAX
    } else {
        candidate_start + model.highlighted() - page
    };
    let frame = Frame {
        preedit: Preedit::plain(model.raw_keys(), model.raw_keys().chars().count()),
        rows,
        highlighted: selected,
        columns: 0,
        column_ems: Vec::new(),
        footer: None,
        sentence: None,
        status: Some(if english {
            "草稿窗 · 英文".into()
        } else {
            "草稿窗 · 中文".into()
        }),
        mode: qingjian_render::Mode {
            english,
            ..Default::default()
        },
        panel: None,
        actions: vec![
            Action {
                target: ActionTarget::InsertTranslation,
                enabled: !writing && model.translation_output().is_some(),
            },
            Action {
                target: ActionTarget::InsertOriginal,
                enabled: !writing && model.original_output().is_some(),
            },
            Action {
                target: ActionTarget::CopyTranslation,
                enabled: model.copy_output().is_some(),
            },
            Action {
                target: ActionTarget::CancelDraft,
                enabled: !writing,
            },
        ],
    };
    (frame, candidate_start)
}
