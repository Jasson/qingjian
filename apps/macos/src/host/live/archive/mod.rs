//! 草稿显式存档与恢复；不保存高亮候选，不自动写入或跨应用取回。

use qingjian_core::Engine;
use qingjian_predict::live::{DraftArchive, SavedDraft, TerminalDraft, normalize_terminal_output};

use crate::host::Host;
use crate::imk::secure_input;

pub(super) fn snapshot(engine: &Engine, draft: &TerminalDraft) -> Option<SavedDraft> {
    let application = engine.application().filter(|app| !app.trim().is_empty())?;
    let raw_keys = engine.raw_preedit().text;
    if draft.committed().is_empty() && raw_keys.is_empty() {
        return None;
    }
    Some(SavedDraft {
        application: application.to_owned(),
        committed: draft.committed().to_owned(),
        raw_keys,
    })
}

pub(super) fn restore(
    archive: &mut DraftArchive,
    engine: &mut Engine,
    draft: &mut TerminalDraft,
) -> Option<SavedDraft> {
    if !engine.composition().is_empty() || !draft.source().is_empty() {
        return None;
    }
    let app = engine.application()?.to_owned();
    let saved = archive.take_for(&app)?;
    let valid = normalize_terminal_output(&saved.committed).as_deref()
        == Some(saved.committed.as_str())
        && normalize_terminal_output(&saved.raw_keys).as_deref() == Some(saved.raw_keys.as_str());
    if valid {
        engine.set_input(&saved.raw_keys);
        if engine.raw_preedit().text == saved.raw_keys && draft.commit(&saved.committed) {
            return Some(saved);
        }
    }
    engine.clear();
    draft.reset();
    archive.stash(&saved.application, &saved.committed, &saved.raw_keys);
    None
}

fn retain(archive: &mut DraftArchive, saved: &SavedDraft, raw_policy_unchanged: bool) {
    if raw_policy_unchanged {
        archive.stash(&saved.application, &saved.committed, &saved.raw_keys);
    } else {
        archive.stash(
            &saved.application,
            &format!("{}{}", saved.committed, saved.raw_keys),
            "",
        );
    }
}

impl Host {
    pub fn suspend_draft(&mut self, reason: &'static str) {
        if secure_input::enabled() || self.engine.application() == Some("com.apple.loginwindow") {
            self.live.archive.clear();
            self.cancel_draft(reason);
            return;
        }
        if self.live.draft_configured_in(self.engine.application())
            || !self.live.draft.source().is_empty()
        {
            if let Some(saved) = snapshot(&self.engine, &self.live.draft) {
                self.live
                    .archive
                    .stash(&saved.application, &saved.committed, &saved.raw_keys);
            }
            self.engine.clear();
            self.cancel_prediction();
            self.reset_session(None, Vec::new());
            self.window.hide();
        }
        self.reset_live_translation(reason);
    }

    pub fn can_restore_from_menu(&self) -> bool {
        !crate::imk::editor::session_owned()
            && crate::imk::controller_active()
            && self.translation.is_none()
            && self.engine.symbol_panel().is_none()
            && self.can_restore_draft()
            && self.engine.composition().is_empty()
            && self.live.draft.source().is_empty()
    }

    pub fn can_restore_draft(&self) -> bool {
        self.draft_active()
            && self
                .engine
                .application()
                .is_some_and(|app| self.live.archive.has_for(app))
    }

    pub fn restore_draft(&mut self) -> Option<(SavedDraft, super::ObservationToken)> {
        if !self.can_restore_draft()
            || !self.engine.composition().is_empty()
            || !self.live.draft.source().is_empty()
        {
            return None;
        }
        self.reset_live_translation("explicit_draft_restore");
        let Some(saved) = restore(
            &mut self.live.archive,
            &mut self.engine,
            &mut self.live.draft,
        ) else {
            self.live_action_notice("草稿未恢复，原文仍保存在当前应用的恢复列表中");
            return None;
        };
        if let Some(speech) = &mut self.live.speech {
            speech.baseline(self.live.draft.committed());
        }
        self.live.automatic.restore();
        self.refresh_draft_candidates();
        self.live_action_notice("草稿已恢复到青简，请确认输入位置；未向应用填入文字");
        Some((saved, self.live_observation_token()))
    }

    /// 写入前最终门禁拒绝时保留已消费的用户原文，不自动重试输出。
    pub fn retain_unwritten_draft(&mut self, saved: &SavedDraft, raw_generation: u64) {
        self.retain_saved_draft(saved, raw_generation);
        self.live_action_notice("文字未填入，已保留待恢复草稿；请返回原应用后明确恢复");
    }

    pub fn retain_saved_draft(&mut self, saved: &SavedDraft, raw_generation: u64) {
        if secure_input::enabled()
            || self.engine.application() == Some("com.apple.loginwindow")
            || !self.settings.config().live_translation.enabled
        {
            self.live.archive.clear();
            return;
        }
        retain(
            &mut self.live.archive,
            saved,
            self.live.raw_generation == raw_generation,
        );
    }

    pub fn draft_snapshot(&self) -> Option<SavedDraft> {
        snapshot(&self.engine, &self.live.draft)
    }
}

#[cfg(test)]
mod tests;
