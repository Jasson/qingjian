//! 主线程门面：有界提交与代次取消，不在按键回调执行网络请求。
use super::{
    SpeechConfig,
    planner::Planner,
    worker::{Command, spawn},
};
use crate::live::LiveTranslationConfig;
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
    mpsc::{self, Receiver, SyncSender},
};

pub struct SpeechCoordinator {
    planner: Planner,

    sender: SyncSender<Command>,

    responses: Receiver<(u64, Result<(), String>)>,

    generation: Arc<AtomicU64>,

    pending: Arc<AtomicUsize>,

    next_id: u64,
}
impl SpeechCoordinator {
    pub fn new(config: SpeechConfig, translation: LiveTranslationConfig) -> Result<Self, String> {
        let (sender, receiver) = mpsc::sync_channel(20);
        let (outgoing, responses) = mpsc::channel();
        let generation = Arc::new(AtomicU64::new(0));
        let pending = Arc::new(AtomicUsize::new(0));
        spawn(
            config,
            translation,
            receiver,
            outgoing,
            generation.clone(),
            pending.clone(),
        )?;
        Ok(Self {
            planner: Planner::default(),
            sender,
            responses,
            generation,
            pending,
            next_id: 0,
        })
    }

    pub fn baseline(&mut self, text: &str) {
        self.planner.baseline(text);
    }

    /// 与有界输入历史的前缀裁剪同步，不视为用户修改或重读旧句。
    pub fn discard_prefix(&mut self, bytes: usize) {
        self.planner.discard_prefix(bytes);
    }

    pub fn update(&mut self, committed: &str) -> Result<(), String> {
        let (edited, sentences) = self.planner.update(committed);
        if edited {
            self.cancel_jobs();
        }
        for text in sentences {
            if self.pending.load(Ordering::SeqCst) >= 16 {
                return Err("朗读队列已满，后续句子未加入；请稍后继续".into());
            }
            self.next_id = self.next_id.wrapping_add(1);
            let generation = self.generation.load(Ordering::SeqCst);
            self.pending.fetch_add(1, Ordering::SeqCst);
            if self
                .sender
                .try_send(Command::Speak {
                    generation,
                    id: self.next_id,
                    text,
                })
                .is_err()
            {
                self.pending.fetch_sub(1, Ordering::SeqCst);
                return Err("朗读队列繁忙，此句未加入".into());
            }
        }
        Ok(())
    }
    fn cancel_jobs(&mut self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        let _ = self.sender.try_send(Command::Wake);
        // Worker 按代次丢弃队列并取消正在播放的旧 session；不依赖控制消息入队。
    }
    pub fn cancel(&mut self) {
        self.cancel_jobs();
        self.planner.baseline("");
    }

    pub fn busy(&self) -> bool {
        self.pending.load(Ordering::SeqCst) != 0
    }

    pub fn poll(&mut self) -> Option<Result<(), String>> {
        while let Ok((generation, result)) = self.responses.try_recv() {
            if generation == self.generation.load(Ordering::SeqCst) {
                return Some(result);
            }
        }
        None
    }
}
impl Drop for SpeechCoordinator {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        let _ = self.sender.try_send(Command::Wake);
    }
}
