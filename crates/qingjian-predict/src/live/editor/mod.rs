//! 可聚焦草稿窗的纯输入与译文状态，平台只临时传入隔离的 Core 会话。

mod input;

use qingjian_core::{Candidate, Engine};

use super::{TerminalDraft, Translation};

#[derive(Debug, Default)]
pub struct FocusedDraftModel {
    draft: TerminalDraft,

    candidates: Vec<Candidate>,

    highlighted: usize,

    raw_keys: String,

    /// 原始键或翻译配置可能改变而预译源文不变，仍须使此前的请求失效。
    extra_revision: u64,

    translation_ready: bool,

    latest: Option<(u64, u64, String)>,

    receipt: Option<Translation>,

    error: Option<String>,
}

impl FocusedDraftModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn source(&self) -> String {
        self.draft.source()
    }

    pub fn committed(&self) -> &str {
        self.draft.committed()
    }

    pub fn raw_keys(&self) -> &str {
        &self.raw_keys
    }

    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    pub fn highlighted(&self) -> usize {
        self.highlighted
    }

    pub fn revision(&self) -> u64 {
        self.draft.revision().wrapping_add(self.extra_revision)
    }

    pub fn refresh(&mut self, engine: &mut Engine) {
        let raw_keys = engine.raw_preedit().text;
        if self.raw_keys != raw_keys {
            self.extra_revision = self.extra_revision.wrapping_add(1);
            self.raw_keys = raw_keys;
        }
        self.candidates = engine
            .query()
            .map(|mut query| {
                engine.annotate(&mut query.candidates);
                query.candidates.items
            })
            .unwrap_or_default();
        self.highlighted = self
            .highlighted
            .min(self.candidates.len().saturating_sub(1));
        self.preview_highlighted();
    }

    pub fn move_highlight(&mut self, delta: isize) {
        self.highlighted = self
            .highlighted
            .saturating_add_signed(delta)
            .min(self.candidates.len().saturating_sub(1));
        self.preview_highlighted();
    }

    fn preview_highlighted(&mut self) {
        self.draft.preview(
            self.candidates
                .get(self.highlighted)
                .map_or("", |candidate| candidate.text.as_str()),
        );
    }

    pub fn submitted(&mut self, id: u64) {
        self.latest = Some((id, self.revision(), self.source()));
        self.translation_ready = false;
        self.error = None;
    }

    /// 配置重载后仅保留旧译文供复制；新服务即使复用请求号也须等新成功回执。
    pub fn invalidate_translation(&mut self) {
        self.extra_revision = self.extra_revision.wrapping_add(1);
        self.latest = None;
        self.translation_ready = false;
        self.error = None;
    }

    pub fn accept(&mut self, reply: Translation) -> bool {
        if !self.latest.as_ref().is_some_and(|(id, revision, source)| {
            *id == reply.id
                && *revision == self.revision()
                && *source == reply.source
                && *source == self.source()
        }) {
            return false;
        }
        match &reply.result {
            Ok(_) => {
                self.receipt = Some(reply);
                self.error = None;
                self.translation_ready = true;
            }
            Err(error) => {
                self.error = Some(error.clone());
                self.translation_ready = false;
            }
        }
        true
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn copy_output(&self) -> Option<&str> {
        self.receipt
            .as_ref()?
            .result
            .as_deref()
            .ok()
            .filter(|text| !text.trim().is_empty())
    }

    pub fn translation_output(&self) -> Option<String> {
        if !self.translation_ready || !self.raw_keys.is_empty() || self.error.is_some() {
            return None;
        }
        let receipt = self.receipt.as_ref()?;
        let (id, revision, source) = self.latest.as_ref()?;
        let latest = *id == receipt.id && *revision == self.revision() && *source == self.source();
        self.draft
            .translation_output(&receipt.source, self.copy_output()?, latest)
    }

    pub fn original_output(&self) -> Option<String> {
        self.draft.original_output_with_pending(&self.raw_keys)
    }

    /// 目标拒绝写入时不清空草稿；平台确认写入成功后才消费本次源文和译文。
    pub fn finish_output(&mut self, engine: &mut Engine, accepted: bool) {
        if accepted {
            self.reset(engine);
        }
    }

    pub fn reset(&mut self, engine: &mut Engine) {
        engine.clear();
        self.draft.reset();
        self.candidates.clear();
        self.highlighted = 0;
        self.raw_keys.clear();
        self.latest = None;
        self.receipt = None;
        self.error = None;
        self.translation_ready = false;
    }
}

#[cfg(test)]
mod tests;
