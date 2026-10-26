//! 本地实时翻译的会话接线与独立显示；断句和网络请求由 predict 负责。

mod actions;
mod application;
mod archive;
mod display;
mod draft;
mod editing;
mod monitor;
mod observation;
mod raw_policy;
mod receipt;
mod session;
mod speech;
mod ticker;

pub(crate) use application::context_policy;
pub use observation::ObservationToken;
pub use receipt::SuccessReceipt;
pub use session::LiveSession;

use crate::candidates::{Frame, Row};
use crate::host::Host;

impl Host {
    pub fn reset_live_translation(&mut self, reason: &'static str) {
        if reason == "secure_input" || reason == "login_window" {
            self.live.archive.clear();
        }
        if reason == "secure_input"
            || reason == "login_window"
            || !self.live.draft.source().is_empty()
            || self.live.draft_configured_in(self.engine.application())
        {
            self.engine.clear();
            self.reset_session(None, Vec::new());
            self.window.hide();
        }
        self.live.reset(reason);
    }

    pub fn pause_live_translation(&mut self, reason: &'static str) {
        if self.draft_active() {
            self.suspend_draft(reason);
            self.refresh_live_actions();
            return;
        }
        self.live.pause(reason);
        self.render_live_translation();
    }

    /// 记录已确认的原文；草稿模式只暂存，不能把候选预览重复计入。
    pub fn commit_live_text(&mut self, text: &str) {
        if !self.live.active() || text.is_empty() {
            return;
        }
        self.live.resume();
        if self.draft_active() {
            if !self.live.automatic.observe(text) {
                self.render_live_translation();
                return;
            }
            if !self.live.draft.commit(text) {
                self.live_action_notice("草稿不接受控制字符，请输入单行文字");
                return;
            }
        } else {
            let previous = self.live.tracker.committed().len();
            self.live.tracker.commit(text);
            let removed = previous + text.len() - self.live.tracker.committed().len();
            if removed > 0
                && let Some(speech) = &mut self.live.speech
            {
                speech.discard_prefix(removed);
            }
        }
        self.update_speech_text();
        self.submit_live_translation();
    }

    pub fn backspace_live_text(&mut self) {
        if self.draft_active() {
            self.backspace_draft();
        } else if self.live.active() && !self.live.paused {
            self.live.tracker.backspace();
            self.update_speech_text();
            self.submit_live_translation();
        }
    }

    /// 导航与查询都会走这里，因此高亮候选变化也立即使旧请求失效。
    pub fn preview_live_translation(&mut self) {
        if !self.live.active() || self.translation.is_some() {
            return;
        }
        if self.live.paused && self.engine.composition().is_empty() {
            self.render_live_translation();
            return;
        }
        self.live.resume();
        let preview = if self.engine.composition().is_empty() {
            String::new()
        } else {
            self.session
                .candidate(self.session.highlighted)
                .map(|candidate| candidate.text)
                .unwrap_or_else(|| self.engine.composition().text().to_owned())
        };
        if self.draft_active() {
            self.live
                .automatic
                .observe(self.engine.composition().text());
            if !self.live.automatic.observe(&preview) {
                return;
            }
            self.live.draft.preview(&preview);
        } else {
            self.live.tracker.preview(&preview);
        }
        self.submit_live_translation();
    }

    fn submit_live_translation(&mut self) {
        let (source, sentence_start) = if self.draft_active() {
            (self.live.draft.source(), None)
        } else {
            let (source, start) = self.live.tracker.source_with_start();
            (source, Some(start))
        };
        let composing = !self.engine.composition().is_empty();
        if let Some(start) = sentence_start {
            self.live
                .automatic
                .observe_sentence(&source, self.engine.composition().text(), start);
        }
        if composing && self.engine.question_mode() {
            self.live.automatic.restore();
        }
        self.live.automatic.finish(&source, composing);
        if source == self.live.source {
            self.render_live_translation();
            return;
        }
        self.live.source = source.clone();
        self.live.action_notice = None;
        self.live.display.begin(self.live.unavailable.as_deref());
        if source.trim().is_empty()
            || !self
                .live
                .automatic
                .visible(&source, &self.live.draft.source(), composing)
        {
            if let Some(translator) = &mut self.live.translator {
                translator.cancel();
            }
            self.live.monitor.stop();
            self.live.display.reset();
            self.render_live_translation();
            return;
        }
        if let Some(translator) = &mut self.live.translator {
            let source_chars = source.chars().count();
            let request_id = translator.submit(source);
            self.live.request_id = Some(request_id);
            tracing::debug!(target: "live_translation", request_id, source_chars, epoch = self.live.epoch, "实时翻译请求");
            self.live.monitor.start();
        }
        self.render_live_translation();
    }

    pub fn poll_live_translation(&mut self) -> bool {
        if crate::imk::secure_input::enabled() {
            self.reset_live_translation("secure_input");
            return false;
        }
        if self.live.paused {
            return false;
        }
        let result = self
            .live
            .translator
            .as_mut()
            .and_then(|worker| worker.poll());
        if let Some(result) = result
            && result.source == self.live.source
            && Some(result.id) == self.live.request_id
        {
            tracing::debug!(target: "live_translation", request_id = result.id, source_chars = result.source.chars().count(), result_chars = result.result.as_ref().map_or(0, |text| text.chars().count()), success = result.result.is_ok(), epoch = self.live.epoch, "实时翻译结果");
            let result = result.result.map(|text| SuccessReceipt {
                text,
                source: result.source,
                request_id: result.id,
                epoch: self.live.epoch,
            });
            self.live.display.complete(result);
            self.live.monitor.stop();
            self.render_live_translation();
            return true;
        }
        false
    }

    fn render_live_translation(&mut self) {
        self.menu.sync_restore(self.can_restore_from_menu());
        if crate::imk::editor::session_owned() {
            self.window.hide();
            self.live.window.hide();
            return;
        }
        let draft = self.draft_active();
        let empty = self.live.source.is_empty();
        let restorable = self.can_restore_draft();
        let shortcut_conflict = draft && self.settings.config().shortcut.draft_shortcuts_conflict();
        let can_open_editor = self.live.active() && self.engine.application().is_some();
        let visible = self.live.automatic.visible(
            &self.live.source,
            &self.live.draft.source(),
            !self.engine.composition().is_empty(),
        );
        if !visible || !self.live.active() {
            self.live.window.hide();
            return;
        }
        let (text, status) = if !empty && self.live.source.trim().is_empty() {
            ("原文仅含空白，可继续输入或取消", "翻译草稿模式")
        } else if empty {
            (
                if restorable {
                    "当前应用有草稿可恢复"
                } else {
                    "请先输入原文草稿"
                },
                "翻译草稿模式",
            )
        } else {
            self.live.display.presentation()
        };
        let mut system_notice = if !draft {
            None
        } else {
            match (
                self.live.backspace_tap_notice.as_deref(),
                self.live.system_hotkey_notice.as_deref(),
            ) {
                (Some(backspace), Some(hotkey)) => Some(format!("{backspace} · {hotkey}")),
                (Some(notice), None) | (None, Some(notice)) => Some(notice.to_owned()),
                (None, None) => None,
            }
        };
        if shortcut_conflict {
            let conflict = "草稿快捷键冲突，输出原文与取消快捷键暂不可用，请修改快捷键设置";
            system_notice = Some(system_notice.map_or_else(
                || conflict.to_owned(),
                |notice| format!("{conflict} · {notice}"),
            ));
        }
        let status = system_notice
            .as_deref()
            .or(self.live.action_notice.as_deref())
            .unwrap_or(status);
        let status = self
            .live_replace_hint()
            .map_or_else(|| status.to_owned(), |hint| format!("{status} · {hint}"));
        let mut actions = if draft {
            vec![
                qingjian_render::Action {
                    target: qingjian_render::ActionTarget::InsertTranslation,
                    enabled: self.draft_output(false).is_some(),
                },
                qingjian_render::Action {
                    target: qingjian_render::ActionTarget::InsertOriginal,
                    enabled: self.draft_output(true).is_some(),
                },
                qingjian_render::Action {
                    target: qingjian_render::ActionTarget::CopyTranslation,
                    enabled: self.live_copy_text().is_some(),
                },
                qingjian_render::Action {
                    target: qingjian_render::ActionTarget::CancelDraft,
                    enabled: true,
                },
            ]
        } else {
            vec![
                qingjian_render::Action {
                    target: qingjian_render::ActionTarget::CopyTranslation,
                    enabled: self.live_copy_text().is_some(),
                },
                qingjian_render::Action {
                    target: qingjian_render::ActionTarget::ReplaceOriginal,
                    enabled: self.live_replace_enabled(),
                },
            ]
        };
        if can_open_editor {
            actions.push(qingjian_render::Action {
                target: qingjian_render::ActionTarget::OpenTranslationDraft,
                enabled: true,
            });
        }
        if restorable {
            actions.push(qingjian_render::Action {
                target: qingjian_render::ActionTarget::RestoreDraft,
                enabled: self.engine.composition().is_empty()
                    && self.live.draft.source().is_empty(),
            });
        }
        let lines = if draft {
            let mut lines = qingjian_render::translation_lines(
                &format!("原文草稿：{}", self.live.draft.source()),
                48,
                3,
            );
            let pending = self.engine.composition().text();
            if !pending.is_empty() {
                lines.extend(qingjian_render::translation_lines(
                    &crate::host::presenting::preedit::pending_input_line(
                        pending,
                        self.engine.composition().cursor(),
                    ),
                    48,
                    1,
                ));
            }
            lines.extend(qingjian_render::translation_lines(
                &format!("译文：{text}"),
                48,
                if pending.is_empty() { 5 } else { 4 },
            ));
            lines
        } else {
            qingjian_render::translation_lines(text, 48, 8)
        };
        let rows = lines
            .into_iter()
            .map(|text| Row {
                index: String::new(),
                text,
                annotation: Vec::new(),
                cloud: false,
            })
            .collect();
        self.live.window.show_translation(
            Frame {
                rows,
                highlighted: usize::MAX,
                footer: Some(status),
                actions,
                ..Frame::default()
            },
            self.anchor,
            self.window.visible_frame(),
        );
    }
}
