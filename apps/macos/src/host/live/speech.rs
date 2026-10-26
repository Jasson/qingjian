//! 朗读会话接线；只接收输入法确认原文，独立轮询不影响文字翻译。
use super::LiveSession;
use crate::host::Host;
use qingjian_predict::live::LiveTranslationConfig;
use qingjian_predict::speech::{SpeechConfig, SpeechCoordinator};

impl LiveSession {
    pub fn configure_speech(
        &mut self,
        speech: &SpeechConfig,
        translation: &LiveTranslationConfig,
        force: bool,
    ) {
        let config = (speech.clone(), translation.clone());
        if !force && self.speech_applied.as_ref() == Some(&config) {
            return;
        }
        self.cancel_speech();
        self.speech = None;
        if speech.enabled && translation.enabled {
            match SpeechCoordinator::new(speech.clone(), translation.clone()) {
                Ok(mut worker) => {
                    worker.baseline(if self.draft.committed().is_empty() {
                        self.tracker.committed()
                    } else {
                        self.draft.committed()
                    });
                    self.speech = Some(worker);
                }
                Err(error) => {
                    self.action_notice = Some(error.clone());
                    tracing::warn!(target: "speech", %error, "译文朗读未启用");
                }
            }
        }
        self.speech_applied = Some(config);
    }

    pub(super) fn cancel_speech(&mut self) {
        if let Some(speech) = &mut self.speech {
            speech.cancel();
        }
        self.speech_monitor.stop();
    }
}
impl Host {
    pub(super) fn update_speech_text(&mut self) {
        let committed = if self.draft_active() {
            self.live.draft.committed()
        } else {
            self.live.tracker.committed()
        }
        .to_owned();
        if let Some(speech) = &mut self.live.speech {
            if let Err(error) = speech.update(&committed) {
                self.live.action_notice = Some(error);
            }
            if speech.busy() {
                self.live.speech_monitor.start();
            }
        }
    }

    pub fn poll_speech(&mut self) {
        if crate::imk::secure_input::enabled() {
            self.live.cancel_speech();
            return;
        }
        let mut notice = None;
        if let Some(speech) = &mut self.live.speech {
            while let Some(result) = speech.poll() {
                if let Err(error) = result {
                    notice = Some(error);
                }
            }
            if !speech.busy() {
                // Worker 先交付结果后归零；覆盖首次 poll 与 busy 检查之间完成的任务。
                while let Some(result) = speech.poll() {
                    if let Err(error) = result {
                        notice = Some(error);
                    }
                }
                self.live.speech_monitor.stop();
            }
        }
        if let Some(error) = notice {
            tracing::warn!(target: "speech", %error, "译文朗读失败");
            self.live_action_notice(&error);
        }
    }
}
