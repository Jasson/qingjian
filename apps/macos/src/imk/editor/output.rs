//! 原目标自动验证与显式新目标确认分开；写请求前先占用待输出，重入不会重复发送。

use super::pending::Pending;
use super::target::Target;

static NEXT_PENDING: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub(super) fn request(original: bool) {
    super::STATE.with(|state| {
        let Ok(mut state) = state.try_borrow_mut() else {
            return;
        };
        let Some(runtime) = state.as_mut() else {
            return;
        };
        if runtime.writing || !runtime.window.owns_keyboard() {
            return;
        }
        let text = if original {
            runtime.model.original_output()
        } else {
            runtime.model.translation_output()
        };
        let Some(text) = text else {
            runtime.status = "请先完成选字并等待当前译文，或选择填入原文".into();
            runtime.render();
            return;
        };
        runtime.pending = Some(Pending {
            id: NEXT_PENDING.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            text,
            original,
            revision: runtime.model.revision(),
            attempts: 0,
            manual: false,
            claimed: false,
        });
        super::owner::Owner::Pending(runtime.origin_pid).set("output_requested");
        runtime.status = "请松开确认键后填入；位置无法确认时将保留草稿".into();
        runtime.render();
        if runtime.held_keys.is_empty() {
            super::KEYBOARD_OWNED.set(false);
            runtime.window.hide();
        }
    });
}

pub(super) fn attempt() {
    let available = super::STATE.with(|state| {
        let Ok(mut state) = state.try_borrow_mut() else {
            return None;
        };
        let runtime = state.as_mut()?;
        let pending = runtime.pending.as_mut()?;
        if runtime.writing
            || pending.manual
            || !runtime.held_keys.is_empty()
            || runtime.window.owns_keyboard()
        {
            return None;
        }
        if !pending.matches(&runtime.model) {
            runtime.pending = None;
            runtime.window.hide();
            super::owner::Owner::Idle.set("stale_confirmation");
            return None;
        }
        pending.attempts = pending.attempts.saturating_add(1);
        if pending.attempts < 3 {
            return None;
        }
        if runtime.target.is_none() {
            pending.manual = true;
            runtime.status = "未填入：打开时未确认目标。请回到输入框，再按开窗快捷键明确确认当前位置，或复制译文".into();
            runtime.render();
            runtime.window.show_notice();
            return None;
        }
        let text = pending.claim(&runtime.model)?;
        let target = runtime.target.take()?;
        runtime.writing = true;
        Some((runtime.id, pending.id, target, text))
    });
    let Some((runtime_id, pending_id, target, text)) = available else {
        return;
    };
    let requested = target.insert(&text, false, || still_writing(runtime_id, pending_id));
    complete(runtime_id, pending_id, target, requested);
}

/// 只有用户在提示后再次明确按开窗键，才允许选择本次新捕获的目标。
pub(super) fn confirm_current(target: Target) -> bool {
    let text = super::STATE.with(|state| {
        let Ok(mut state) = state.try_borrow_mut() else {
            return None;
        };
        let runtime = state.as_mut()?;
        if runtime.writing {
            return None;
        }
        runtime.prune_released_keys();
        if !runtime.held_keys.is_empty() {
            return None;
        }
        let pending = runtime.pending.as_mut()?;
        if !pending.manual {
            return None;
        }
        let text = pending.claim(&runtime.model)?;
        runtime.writing = true;
        Some((runtime.id, pending.id, text))
    });
    let Some((runtime_id, pending_id, text)) = text else {
        return false;
    };
    let requested = target.insert(&text, true, || still_writing(runtime_id, pending_id));
    complete(runtime_id, pending_id, target, requested);
    true
}

fn complete(runtime_id: u64, pending_id: u64, target: Target, requested: bool) {
    super::STATE.with(|state| {
        let Ok(mut state) = state.try_borrow_mut() else {
            return;
        };
        let Some(runtime) = state.as_mut() else {
            return;
        };
        if runtime.id != runtime_id {
            return;
        }
        if !runtime.writing
            || !runtime.pending.as_ref().is_some_and(|pending| {
                pending.id == pending_id && pending.claimed && pending.matches(&runtime.model)
            })
        {
            if runtime.target.is_none() {
                runtime.target = Some(target);
            }
            return;
        }
        runtime.writing = false;
        runtime.target = Some(target);
        if requested {
            runtime.pending = None;
            // 写请求一旦发出就消耗本地正文，不能依赖可能重入借用失败的 Host。
            runtime.session.discard_input();
            runtime.model = qingjian_predict::live::FocusedDraftModel::new();
            runtime.submitted_revision = None;
            if let Some(translator) = runtime.translator.as_mut() {
                translator.cancel();
            }
            runtime.status = "已请求填入文字；没有发送回车或消息".into();
            runtime.window.hide();
            super::owner::Owner::Idle.set("output_completed");
        } else {
            super::owner::Owner::Pending(runtime.origin_pid).set("output_rejected");
            if let Some(pending) = runtime.pending.as_mut() {
                pending.rejected();
            }
            runtime.status = if let Some(combo) = runtime.shortcut.open_translation_draft {
                format!("未填入。回到要填入的输入框，再按 {combo} 明确确认当前新位置；也可复制译文")
            } else {
                "未填入。开窗快捷键已禁用，可先复制译文，或在设置中启用开窗键后再确认位置".into()
            };
            runtime.render();
            runtime.window.show_notice();
        }
    });
}

fn still_writing(runtime_id: u64, pending_id: u64) -> bool {
    if super::CLOSE_PENDING.get() {
        return false;
    }
    super::STATE.with(|state| {
        state.try_borrow().ok().is_some_and(|state| {
            state.as_ref().is_some_and(|runtime| {
                runtime.id == runtime_id
                    && runtime.writing
                    && runtime.pending.as_ref().is_some_and(|pending| {
                        pending.id == pending_id
                            && pending.claimed
                            && pending.matches(&runtime.model)
                    })
            })
        })
    })
}
