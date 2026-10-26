//! 实时翻译的复制与手动替换；只操作当前已核验会话，不模拟按键删除。

use objc2_foundation::NSRange;
use qingjian_platform::{KeyCombo, Modifiers};
use qingjian_render::ActionTarget;

use super::QingjianInputController;
use super::output_guard::OutputGuard;
use crate::host;
use crate::imk::{TextClient, secure_input};

impl QingjianInputController {
    pub(super) fn handle_live_shortcut(
        &self,
        pressed: Modifiers,
        typed: Option<char>,
        client: TextClient<'_>,
        repeat: bool,
    ) -> Option<bool> {
        let action = host::with(|h| {
            if !h.live.active() {
                return None;
            }
            let shortcut = &h.settings.config().shortcut;
            let matches = |combo| shortcut_matches(combo, pressed, typed);
            if !shortcut.open_translation_draft_conflicts()
                && matches(shortcut.open_translation_draft)
            {
                return Some(ActionTarget::OpenTranslationDraft);
            }
            // 手改配置也不能抢已有选区翻译与按需联想的组合键。
            if matches(Some(shortcut.translate_selection)) || matches(Some(shortcut.predict)) {
                return None;
            }
            if h.draft_active() && matches(shortcut.insert_live_translation) {
                Some(ActionTarget::InsertTranslation)
            } else if matches(shortcut.copy_live_translation) && h.live_copy_text().is_some() {
                Some(ActionTarget::CopyTranslation)
            } else if matches(shortcut.replace_live_translation)
                && !h.draft_active()
                && h.settings.config().live_translation.allow_replace
                && !h.live.source.is_empty()
            {
                Some(ActionTarget::ReplaceOriginal)
            } else {
                None
            }
        })
        .flatten()?;
        if action == ActionTarget::InsertTranslation {
            let combo =
                host::with(|h| h.settings.config().shortcut.insert_live_translation).flatten();
            if combo.is_some_and(crate::imk::hotkey::owns) {
                return Some(true);
            }
            tracing::info!(target: "live_translation", source = "imk", reason = "fallback", "收到译文输出快捷键");
        }
        if action == ActionTarget::OpenTranslationDraft {
            return Some(
                repeat
                    || crate::imk::editor::open(client, crate::imk::editor::OpenSource::Shortcut),
            );
        }
        Some(self.perform_live_action(action, Some(client)))
    }

    pub(super) fn perform_live_action(
        &self,
        action: ActionTarget,
        client: Option<TextClient<'_>>,
    ) -> bool {
        self.perform_live_action_checked(action, client, None)
    }

    pub(super) fn perform_registered_live_action(
        &self,
        client: TextClient<'_>,
        generation: u32,
        observation: &host::ObservationToken,
    ) -> bool {
        self.perform_live_action_checked(
            ActionTarget::InsertTranslation,
            Some(client),
            Some(OutputGuard::Carbon(generation, observation)),
        )
    }

    pub(super) fn perform_live_action_checked(
        &self,
        action: ActionTarget,
        client: Option<TextClient<'_>>,
        expected: Option<OutputGuard<'_>>,
    ) -> bool {
        if (crate::imk::editor::session_owned() && action != ActionTarget::OpenTranslationDraft)
            || !self.is_current_controller()
            || secure_input::enabled()
        {
            return false;
        }
        if action == ActionTarget::OpenTranslationDraft {
            return client.is_some_and(|client| {
                crate::imk::editor::open(client, crate::imk::editor::OpenSource::Button)
            });
        }
        if let Some(client) = client
            && !self.sync_live_application(client)
        {
            return false;
        }
        if let Some(expected) = expected
            && host::with(|h| expected.valid(h)) != Some(true)
        {
            return true;
        }
        match action {
            ActionTarget::CopyTranslation => {
                let copied_text =
                    host::with(|h| h.live_copy_text().map(|text| (text, h.live.epoch))).flatten();
                let Some((text, epoch)) = copied_text else {
                    return false;
                };
                // 剪贴板操作放在 Host 借用之外，不要求读应用文本。
                let copied = host::copy_text_to_pasteboard(&text);
                host::with(|h| {
                    if h.live.epoch != epoch {
                        return;
                    }
                    h.live_action_notice(if copied {
                        "已复制译文"
                    } else {
                        "复制失败，请重试"
                    })
                });
                true
            }
            ActionTarget::InsertTranslation | ActionTarget::InsertOriginal => {
                if let Some(client) = client {
                    let _ =
                        self.output_draft(client, action == ActionTarget::InsertOriginal, expected);
                }
                true
            }
            ActionTarget::CancelDraft => {
                host::with(|h| {
                    if h.draft_active() && expected.is_none_or(|guard| guard.valid(h)) {
                        h.cancel_draft("draft_cancelled");
                    }
                });
                true
            }
            ActionTarget::OpenTranslationDraft => false,
            ActionTarget::RestoreDraft => {
                if let Some(client) = client {
                    self.restore_saved_draft(client, expected);
                }
                true
            }
            ActionTarget::ReplaceOriginal => {
                if host::with(|h| h.draft_active()).unwrap_or(true) {
                    host::with(|h| h.live_action_notice("草稿模式请使用输出译文或输出原文"));
                    return true;
                }
                if let Some(client) = client {
                    self.replace_live_translation(client);
                }
                true
            }
        }
    }

    fn replace_live_translation(&self, client: TextClient<'_>) {
        let state = host::with(|h| {
            if h.live.active()
                && h.settings.config().live_translation.allow_replace
                && h.engine.composition().is_empty()
            {
                h.pause_live_translation("selection_replacement");
            }
            h.live_replacement_receipt()
                .map(|receipt| (receipt, h.live_observation_token()))
        })
        .flatten();
        let Some((receipt, observation)) = state else {
            host::with(|h| {
                let reason = h.live_replacement_blocker();
                h.reject_live_replacement(
                    reason,
                    "请先选字上屏，并等待最新译文",
                    false,
                    None,
                    "none",
                );
            });
            return;
        };
        let still_current = || {
            !crate::imk::editor::session_owned()
                && self.is_current_controller()
                && !secure_input::enabled()
                && host::with(|h| {
                    h.live_receipt_is_current(&receipt)
                        && observation.matches(&h.live_observation_token())
                })
                .unwrap_or(false)
        };
        if !still_current() {
            return;
        }
        if host::with(|h| h.live_terminal_blocked(None)).unwrap_or(true) {
            host::with(|h| {
                h.reject_live_replacement(
                    "terminal_readonly_selection",
                    "终端选区不支持直接替换，请复制译文",
                    false,
                    None,
                    "none",
                )
            });
            return;
        }
        let Some(first_metadata) = client.live_selection_metadata(still_current) else {
            return;
        };
        if !still_current() {
            return;
        }
        let accepted =
            host::with(|h| h.accept_live_application(first_metadata.application.as_deref()))
                .unwrap_or(false);
        if !accepted || !still_current() {
            return;
        }
        if let Some(reason) =
            host::with(|h| h.live_application_blocker(first_metadata.application.as_deref()))
                .flatten()
        {
            host::with(|h| {
                h.reject_live_replacement(
                    reason,
                    if reason == "application_unknown" {
                        "无法识别当前应用，请复制译文"
                    } else {
                        "终端选区不支持直接替换，请复制译文"
                    },
                    false,
                    None,
                    "none",
                )
            });
            return;
        }
        let candidate = host::with(|h| {
            h.live.last_context = Some(first_metadata.clone());
            h.live_replacement_candidate(&receipt, &first_metadata)
        })
        .flatten();
        let Some(candidate) = candidate else {
            host::with(|h| {
                let reason = h.live_replacement_blocker();
                let notice = if reason == "sentence_too_long" {
                    "当前句过长，请分句或复制"
                } else {
                    "请完整选中与当前译文对应的原文"
                };
                h.reject_live_replacement(reason, notice, false, None, "none");
            });
            return;
        };
        if !client.supports_document_access(still_current) {
            if still_current() {
                host::with(|h| {
                    h.reject_live_replacement(
                        "document_access_unsupported",
                        "当前应用不支持范围替换，可复制译文",
                        false,
                        None,
                        "none",
                    )
                });
            }
            return;
        }
        // 仅读取用户当前非空选区，不推算光标之前的句子，也不请求新的模型译文。
        let first = client.read_selected_candidate(&candidate, &first_metadata, still_current);
        if !still_current() {
            return;
        }
        let first_units = first.text.as_ref().map(|text| text.encode_utf16().count());
        let first_verified = first
            .text
            .as_deref()
            .and_then(|text| candidate.verify_exact(text));
        if first_verified.is_none() {
            host::with(|h| {
                h.reject_live_replacement(
                    if first.text.is_none() {
                        "exact_read_unavailable"
                    } else {
                        "exact_read_mismatch"
                    },
                    if first.text.is_none() {
                        "当前应用无法读取选中的原文，可复制译文"
                    } else {
                        "选中内容须与当前译文对应的整段原文完全一致"
                    },
                    first.attempted,
                    first_units,
                    first.reader,
                )
            });
            return;
        }
        let Some(second_metadata) = client.live_selection_metadata(still_current) else {
            return;
        };
        if !still_current() {
            return;
        }
        let same_candidate =
            host::with(|h| h.live_replacement_candidate(&receipt, &second_metadata))
                .flatten()
                .is_some_and(|current| current == candidate)
                && same_replacement_metadata(&first_metadata, &second_metadata);
        if !same_candidate {
            host::with(|h| {
                h.reject_live_replacement(
                    "selection_or_candidate_changed",
                    "输入位置已变化，未替换",
                    true,
                    first_units,
                    first.reader,
                )
            });
            return;
        }
        let second = client.read_selected_candidate(&candidate, &second_metadata, still_current);
        if !still_current() {
            return;
        }
        let second_units = second.text.as_ref().map(|text| text.encode_utf16().count());
        let verified = second
            .text
            .as_deref()
            .and_then(|text| candidate.verify_exact(text));
        let Some(plan) = verified else {
            host::with(|h| {
                h.reject_live_replacement(
                    if second.text.is_none() {
                        "exact_read_unavailable"
                    } else {
                        "exact_read_mismatch"
                    },
                    if second.text.is_none() {
                        "当前应用无法读取选中的原文，可复制译文"
                    } else {
                        "选中内容须与当前译文对应的整段原文完全一致"
                    },
                    second.attempted,
                    second_units,
                    second.reader,
                )
            });
            return;
        };
        if !client.supports_document_access(still_current) {
            if still_current() {
                host::with(|h| {
                    h.reject_live_replacement(
                        "document_access_changed",
                        "当前应用不支持范围替换，可复制译文",
                        true,
                        second_units,
                        second.reader,
                    )
                });
            }
            return;
        }
        let Some(final_metadata) = client.live_selection_metadata(still_current) else {
            return;
        };
        if !still_current() {
            return;
        }
        let accepted =
            host::with(|h| h.accept_live_application(final_metadata.application.as_deref()))
                .unwrap_or(false);
        if !accepted || !still_current() {
            return;
        }
        if let Some(reason) =
            host::with(|h| h.live_application_blocker(final_metadata.application.as_deref()))
                .flatten()
        {
            host::with(|h| {
                h.reject_live_replacement(
                    reason,
                    if reason == "application_unknown" {
                        "无法识别当前应用，请复制译文"
                    } else {
                        "终端选区不支持直接替换，请复制译文"
                    },
                    true,
                    second_units,
                    second.reader,
                )
            });
            return;
        }
        let same_candidate =
            host::with(|h| h.live_replacement_candidate(&receipt, &final_metadata))
                .flatten()
                .is_some_and(|current| current == candidate)
                && same_replacement_metadata(&first_metadata, &final_metadata);
        if !same_candidate {
            host::with(|h| {
                h.reject_live_replacement(
                    "selection_or_candidate_changed",
                    "输入位置已变化，未替换",
                    true,
                    second_units,
                    second.reader,
                )
            });
            return;
        }
        let replacement = plan.replacement(&receipt.text);
        if !still_current() {
            return;
        }
        host::with(|h| {
            tracing::info!(target: "live_translation", action = "replace", write_requested = true,
                reader = second.reader, read_attempted = true, exact_read_units = second_units, exact_reads = 2,
                document_units = final_metadata.document_length, before_range = ?final_metadata.before_range,
                application = final_metadata.application.as_deref(), location_units = plan.location_utf16,
                length_units = plan.length_utf16, request_id = receipt.request_id, epoch = receipt.epoch,
                "实时翻译请求精确范围写入");
            h.live.last_context = Some(final_metadata);
        });
        // 两次实际逐字读取均成功才使用 verified plan；只发一次写入，不模拟选中或退格。
        client.replace_range(
            &replacement,
            NSRange::new(plan.location_utf16, plan.length_utf16),
        );
        host::with(|h| {
            if h.live_receipt_is_current(&receipt)
                && observation.matches(&h.live_observation_token())
            {
                h.engine.clear();
                h.engine.break_chain();
                h.cancel_prediction();
                h.reset_session(None, Vec::new());
                h.window.hide();
                h.reset_live_translation("translation_replace_requested");
            }
        });
    }
}

fn same_replacement_metadata(
    original: &qingjian_predict::live::context::ContextSnapshot,
    current: &qingjian_predict::live::context::ContextSnapshot,
) -> bool {
    original.client_proxy == current.client_proxy
        && original.selected == current.selected
        && original.active_marked().is_none()
        && current.active_marked().is_none()
        && !original.application.as_ref().is_some_and(|application| {
            current
                .application
                .as_ref()
                .is_some_and(|current| application != current)
        })
}

fn shortcut_matches(combo: Option<KeyCombo>, pressed: Modifiers, typed: Option<char>) -> bool {
    combo.is_some_and(|combo| {
        combo.key.is_ascii_alphabetic()
            && !combo.modifiers.is_empty()
            && pressed == combo.modifiers
            && typed == Some(combo.key)
    })
}

#[cfg(test)]
mod tests {
    use super::shortcut_matches;
    use qingjian_platform::KeyCombo;

    #[test]
    fn live_shortcuts_require_modified_letters_and_respect_disable() {
        let copy = KeyCombo {
            key: 'c',
            ..KeyCombo::TRANSLATE_DEFAULT
        };
        assert!(shortcut_matches(Some(copy), copy.modifiers, Some('c')));
        assert!(!shortcut_matches(None, copy.modifiers, Some('c')));
        assert!(!shortcut_matches(Some(copy), copy.modifiers, Some('r')));
        let digit = KeyCombo { key: '1', ..copy };
        assert!(!shortcut_matches(Some(digit), digit.modifiers, Some('1')));
        let unmodified = KeyCombo {
            modifiers: Default::default(),
            ..copy
        };
        assert!(!shortcut_matches(
            Some(unmodified),
            unmodified.modifiers,
            Some('c')
        ));
    }
}
