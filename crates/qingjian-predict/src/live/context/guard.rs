//! 依据范围、真实读取与受控历史判断上下文是否足以触发重置。

use super::{ContextConfidence, ContextDecision, ContextPolicy, ContextRange, ContextSnapshot};

#[derive(Debug, Default)]
pub struct ContextGuard {
    confidence: ContextConfidence,

    application: Option<String>,
}

impl ContextGuard {
    pub fn confidence(&self) -> ContextConfidence {
        self.confidence
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn observe(&mut self, snapshot: &ContextSnapshot, history: &str) -> ContextDecision {
        self.observe_composition(snapshot, history, false)
    }

    /// 宿主没给出活动 marked 范围时，不能将组词中的拼音误认为已上屏前文。
    pub fn observe_composition(
        &mut self,
        snapshot: &ContextSnapshot,
        history: &str,
        composition_active: bool,
    ) -> ContextDecision {
        self.observe_with_policy(
            snapshot,
            history,
            composition_active,
            ContextPolicy::VerifiedDocument,
        )
    }

    pub fn observe_with_policy(
        &mut self,
        snapshot: &ContextSnapshot,
        history: &str,
        composition_active: bool,
        policy: ContextPolicy,
    ) -> ContextDecision {
        let changed_application = self.application.as_ref().is_some_and(|previous| {
            snapshot
                .application
                .as_ref()
                .is_some_and(|current| current != previous)
        });
        let previous = self.confidence;
        if snapshot.application.is_some() {
            self.application = snapshot.application.clone();
        }
        let event_only = policy == ContextPolicy::EventOnly;
        // 事件模式也记录屏幕差异，但不因此声明具备文档能力或清空输入流。
        let (confidence, mismatch) = classify(
            snapshot,
            history,
            if event_only {
                ContextConfidence::Full
            } else {
                previous
            },
            composition_active,
        );
        let confidence = if event_only {
            ContextConfidence::Partial
        } else {
            confidence
        };
        self.confidence = confidence;
        ContextDecision {
            confidence,
            context_mismatch: mismatch,
            reset_reason: if changed_application {
                Some("application_changed")
            } else if !event_only
                && !composition_active
                && snapshot.active_marked().is_none()
                && snapshot
                    .selected
                    .is_some_and(|range| range.length > 0 && range.within(snapshot.document_length))
            {
                Some("selection_changed")
            } else if mismatch && !event_only {
                Some("trusted_context_changed")
            } else {
                None
            },
        }
    }
}

fn classify(
    snapshot: &ContextSnapshot,
    history: &str,
    previous: ContextConfidence,
    composition_active: bool,
) -> (ContextConfidence, bool) {
    let Some(selected) = snapshot
        .selected
        .filter(|range| range.within(snapshot.document_length))
    else {
        return (ContextConfidence::Unreadable, false);
    };
    let active_marked = snapshot.active_marked();
    if active_marked.is_some_and(|range| !range.within(snapshot.document_length)) {
        return (ContextConfidence::Unreadable, false);
    }
    if let Some(marked) = active_marked {
        let selected_end = selected.end().unwrap_or(usize::MAX);
        let marked_end = marked.end().unwrap_or(0);
        if selected.start < marked.start || selected_end > marked_end {
            return (ContextConfidence::Unreadable, false);
        }
    }
    if composition_active && active_marked.is_none() {
        return (ContextConfidence::Partial, false);
    }
    let Some(before_range) = snapshot.before_range else {
        return (ContextConfidence::Unreadable, false);
    };
    let start = active_marked.map_or(selected.start, |range| range.start);
    let expected_range = ContextRange {
        start: start.saturating_sub(snapshot.lookback_limit),
        length: start.min(snapshot.lookback_limit),
    };
    if before_range != expected_range || !before_range.within(snapshot.document_length) {
        return (ContextConfidence::Unreadable, false);
    }
    let Some(before) = snapshot.before.as_ref() else {
        return (ContextConfidence::Unreadable, false);
    };
    let before: Vec<u16> = before.encode_utf16().collect();
    if before.len() != before_range.length {
        return (ContextConfidence::Unreadable, false);
    }
    let Some(probe) = snapshot
        .probe_range
        .filter(|range| range.within(snapshot.document_length))
    else {
        return (ContextConfidence::Unreadable, false);
    };
    if snapshot.probe_length != Some(probe.length) {
        return (ContextConfidence::Unreadable, false);
    }
    let history: Vec<u16> = history.encode_utf16().collect();
    if before.is_empty() {
        // marked 位于局部窗口原点不是“整个文档已清空”的证据。
        if active_marked.is_some() || previous != ContextConfidence::Full {
            return (ContextConfidence::Partial, false);
        }
        return (ContextConfidence::Full, !history.is_empty());
    }
    let overlap = before.len().min(history.len());
    let matches = before[before.len() - overlap..] == history[history.len() - overlap..];
    let covers_history = before.len() >= history.len();
    let complete_window = snapshot.lookback_limit > 0
        && before.len() == snapshot.lookback_limit
        && start >= history.len();
    if history.is_empty() {
        // 没有受控文字时不能仅凭宿主已有内容建立“完整可读”的能力。
        return (ContextConfidence::Partial, false);
    }
    if (covers_history || complete_window) && start >= history.len() {
        if matches || previous == ContextConfidence::Full {
            (ContextConfidence::Full, !matches)
        } else {
            (ContextConfidence::Partial, false)
        }
    } else {
        (ContextConfidence::Partial, false)
    }
}

#[cfg(test)]
#[path = "event_tests.rs"]
mod event_tests;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
