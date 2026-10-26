//! 单独后台线程顺序翻译并等待播放终态；取消可打断 HTTP 和播放等待。
use super::SpeechConfig;
use crate::live::{
    LiveTranslationConfig,
    translator::{local_endpoint, translate},
};
use reqwest::{Client, Url};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
    mpsc::{Receiver, RecvTimeoutError, Sender},
};
use std::time::Duration;
use tokio::task::JoinHandle;

pub(super) enum Command {
    Wake,

    Speak {
        generation: u64,
        id: u64,
        text: String,
    },
}

pub(super) fn spawn(
    config: SpeechConfig,
    translation: LiveTranslationConfig,
    incoming: Receiver<Command>,
    outgoing: Sender<(u64, Result<(), String>)>,
    generation: Arc<AtomicU64>,
    pending: Arc<AtomicUsize>,
) -> Result<(), String> {
    let mut endpoint = local_endpoint(&config.base_url)?;
    endpoint.set_path("/");
    let translation_endpoint = local_endpoint(&translation.base_url)?;
    if !config.speed.is_finite() || !(0.5..=2.0).contains(&config.speed) {
        return Err("朗读速度需介于 0.5 与 2.0".into());
    }
    let client = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_millis(config.timeout_ms.clamp(100, 30000)))
        .build()
        .map_err(|_| "无法创建朗读连接")?;
    let translation_client = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_millis(
            translation.timeout_ms.clamp(100, 120000),
        ))
        .build()
        .map_err(|_| "无法创建朗读翻译连接")?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "无法创建朗读任务")?;
    std::thread::Builder::new()
        .name("qingjian-speech".into())
        .spawn(move || {
            let mut seen_generation = generation.load(Ordering::SeqCst);
            let mut session = uuid::Uuid::new_v4().to_string();
            let mut queue = std::collections::VecDeque::new();
            let mut task: Option<JoinHandle<Result<(), String>>> = None;
            loop {
                let current = generation.load(Ordering::SeqCst);
                if current != seen_generation {
                    if let Some(active) = task.take() {
                        active.abort();
                        pending.fetch_sub(1, Ordering::SeqCst);
                    }
                    for _ in queue.drain(..) {
                        pending.fetch_sub(1, Ordering::SeqCst);
                    }
                    let _ = runtime.block_on(async {
                        client
                            .post(endpoint.join("cancel").unwrap())
                            .json(&json!({"session":session}))
                            .send()
                            .await
                    });
                    session = uuid::Uuid::new_v4().to_string();
                    seen_generation = current;
                }
                let message = if task.is_some() || !queue.is_empty() {
                    incoming.recv_timeout(Duration::from_millis(10))
                } else {
                    incoming.recv().map_err(|_| RecvTimeoutError::Disconnected)
                };
                match message {
                    Ok(Command::Wake) => continue,
                    Ok(Command::Speak {
                        generation: job_generation,
                        id,
                        text,
                    }) => {
                        if job_generation == seen_generation {
                            queue.push_back((id, text));
                        } else {
                            pending.fetch_sub(1, Ordering::SeqCst);
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        if let Some(active) = task.take() {
                            active.abort();
                        }
                        let _ = runtime.block_on(async {
                            client
                                .post(endpoint.join("cancel").unwrap())
                                .json(&json!({"session":session}))
                                .send()
                                .await
                        });
                        break;
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                }
                runtime.block_on(async { tokio::time::sleep(Duration::from_millis(1)).await });
                if task.as_ref().is_some_and(JoinHandle::is_finished) {
                    let result = runtime
                        .block_on(task.take().unwrap())
                        .unwrap_or_else(|_| Err("朗读任务已停止".into()));
                    if result.is_err() {
                        for _ in queue.drain(..) {
                            pending.fetch_sub(1, Ordering::SeqCst);
                        }
                        while let Ok(command) = incoming.try_recv() {
                            if matches!(command, Command::Speak { .. }) {
                                pending.fetch_sub(1, Ordering::SeqCst);
                            }
                        }
                        let _ = runtime.block_on(async {
                            client
                                .post(endpoint.join("cancel").unwrap())
                                .json(&json!({"session":session}))
                                .send()
                                .await
                        });
                        session = uuid::Uuid::new_v4().to_string();
                    }
                    let _ = outgoing.send((seen_generation, result));
                    // 先交付结果再归零，保证主线程停止定时器前一定能看到终态。
                    pending.fetch_sub(1, Ordering::SeqCst);
                }
                if task.is_none()
                    && let Some((id, text)) = queue.pop_front()
                {
                    let (
                        client,
                        translation_client,
                        endpoint,
                        translation_endpoint,
                        model,
                        config,
                        session,
                        generation,
                    ) = (
                        client.clone(),
                        translation_client.clone(),
                        endpoint.clone(),
                        translation_endpoint.clone(),
                        translation.model.clone(),
                        config.clone(),
                        session.clone(),
                        generation.clone(),
                    );
                    let expected = seen_generation;
                    task = Some(runtime.spawn(async move {
                        let text =
                            translate(&translation_client, translation_endpoint, &model, &text)
                                .await?;
                        if generation.load(Ordering::SeqCst) != expected {
                            return Ok(());
                        }
                        play(&client, &endpoint, &config, &session, id, &text).await
                    }));
                }
            }
        })
        .map_err(|_| "无法启动朗读线程".to_owned())?;
    Ok(())
}

async fn play(
    client: &Client,
    endpoint: &Url,
    config: &SpeechConfig,
    session: &str,
    id: u64,
    text: &str,
) -> Result<(), String> {
    let id = id.to_string();
    let response = client.post(endpoint.join("speak").unwrap()).json(&json!({"session":session,"id":id,"text":text,"voice":config.voice,"speed":config.speed,"language":"auto"})).send().await.map_err(|_| "无法连接本机朗读服务，请先启动 TTS 服务")?;
    if response.status().as_u16() != 202 {
        return Err(format!(
            "朗读请求失败（HTTP {}）",
            response.status().as_u16()
        ));
    }
    for _ in 0..600 {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let response = client
            .get(endpoint.join("status").unwrap())
            .query(&[("session", session), ("id", id.as_str())])
            .send()
            .await
            .map_err(|_| "朗读服务状态读取失败")?;
        if !response.status().is_success() {
            return Err("朗读服务未确认播放状态".into());
        }
        let value: Value = response.json().await.map_err(|_| "朗读服务返回格式无效")?;
        match value.get("state").and_then(Value::as_str) {
            Some("completed") => return Ok(()),
            Some("failed") => return Err("本机语音合成或播放失败，请查看 TTS 日志".into()),
            Some("cancelled") => return Ok(()),
            Some("queued" | "generating" | "playing") => {}
            _ => return Err("朗读服务返回未知状态".into()),
        }
    }
    let _ = client
        .post(endpoint.join("cancel").unwrap())
        .json(&json!({"session":session}))
        .send()
        .await;
    Err("朗读等待超时，已取消此会话".into())
}
