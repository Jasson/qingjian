//! 草稿确认填入：身份与输入双重核验，只发一次纯文本写入，不发送回车。

use super::QingjianInputController;
use super::output_guard::OutputGuard;
use crate::host;
use crate::imk::{TextClient, frontmost, secure_input};

impl QingjianInputController {
    pub(super) fn output_draft(
        &self,
        client: TextClient<'_>,
        original: bool,
        expected: Option<OutputGuard<'_>>,
    ) -> Option<bool> {
        if crate::imk::editor::session_owned() {
            return Some(false);
        }
        let generation = expected.map(OutputGuard::generation);
        let state = host::with(|h| {
            if expected.is_some_and(|guard| !guard.valid(h)) {
                tracing::info!(target: "live_translation", generation, reason = "input_changed", "草稿输出未执行");
                return Err(());
            }
            let output = h.draft_output(original);
            if output.is_none() {
                tracing::info!(target: "live_translation", generation, reason = "draft_not_ready", "草稿输出未执行");
                h.live_action_notice(if original {
                    "原文草稿为空或含有不支持的控制字符，未输出"
                } else {
                    "请先选定全部原文，并等待当前整段译文；也可明确输出原文"
                });
            }
            Ok(output.and_then(|text| {
                h.draft_snapshot().map(|saved| {
                    (
                        text,
                        h.live_observation_token(),
                        saved,
                        h.live.raw_generation,
                    )
                })
            }))
        })?;
        let Ok(state) = state else { return Some(false) };
        let Some((text, observation, saved, raw_generation)) = state else {
            return Some(true);
        };
        let application = client.bundle_identifier();
        let foreground = frontmost::matches(observation.application());
        if crate::imk::editor::session_owned()
            || !foreground
            || !self.is_current_controller()
            || secure_input::enabled()
            || application.as_deref() != observation.application()
        {
            return Some(false);
        }
        let consumed = host::with(|h| {
            if expected.is_some_and(|guard| !guard.valid(h)) {
                return None;
            }
            if crate::imk::editor::session_owned()
                || !h.draft_active()
                || h.engine.application() != application.as_deref()
                || !observation.matches(&h.live_observation_token())
                || h.draft_output(original).as_deref() != Some(text.as_str())
            {
                return None;
            }
            // 先消费草稿并使旧请求与点击失效，insertText 重入时不会再次输出同一段。
            h.cancel_draft("draft_output_requested");
            if let Some(OutputGuard::Tap(generation, serial, _)) = expected {
                crate::imk::backspace_tap::action_consumed(generation, serial);
            }
            Some((h.live_observation_token(), h.settings.config().shortcut))
        })?;
        let Some((consumed_observation, consumed_shortcut)) = consumed else {
            return Some(false);
        };
        let foreground = frontmost::matches(observation.application());
        let unchanged = foreground
            && host::with(|h| {
                h.draft_active()
                    && h.engine.application() == application.as_deref()
                    && consumed_observation.matches(&h.live_observation_token())
                    && consumed_shortcut == h.settings.config().shortcut
            })
            .unwrap_or(false);
        let requested = if unchanged
            && !crate::imk::editor::session_owned()
            && self.is_current_controller()
            && !secure_input::enabled()
        {
            let requested = client.insert_confirmed_draft(&text);
            tracing::info!(target: "live_translation", original, write_requested = requested,
                "草稿确认填入");
            requested
        } else {
            false
        };
        host::with(|h| {
            if !requested {
                h.retain_unwritten_draft(&saved, raw_generation);
            } else if consumed_observation.matches(&h.live_observation_token()) && h.draft_active()
            {
                h.live_action_notice("已填入，下一次回车由应用处理");
            }
        });
        Some(true)
    }

    pub(super) fn output_tapped_original(
        &self,
        client: TextClient<'_>,
        guard: OutputGuard<'_>,
    ) -> Option<bool> {
        self.output_draft(client, true, Some(guard))
    }

    pub(super) fn sync_live_application(&self, client: TextClient<'_>) -> bool {
        let observation = host::with(|h| h.live_observation_token());
        let Some(observation) = observation else {
            return false;
        };
        let application = client.bundle_identifier();
        if !self.is_current_controller() || secure_input::enabled() {
            return false;
        }
        host::with(|h| {
            if !observation.matches(&h.live_observation_token()) {
                return false;
            }
            h.accept_live_application(application.as_deref())
        })
        .unwrap_or(false)
    }
}
