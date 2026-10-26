//! 显式启用的本机服务联调，不在常规测试连接外部进程。
use crate::live::LiveTranslationConfig;
use crate::speech::{SpeechConfig, SpeechCoordinator};
use std::thread;
use std::time::{Duration, Instant};
#[test]
#[ignore = "手动验证本机已启动的 Ollama 与 --no-play TTS，不在常规测试访问服务"]
fn real_local_coordinator_probe_no_play() {
    assert_eq!(
        std::env::var("QINGJIAN_TTS_NO_PLAY_CONFIRMED").as_deref(),
        Ok("1")
    );
    let mut worker = SpeechCoordinator::new(
        SpeechConfig::default(),
        LiveTranslationConfig {
            timeout_ms: 60000,
            ..Default::default()
        },
    )
    .unwrap();
    worker.update("你好，欢迎学习英语。").unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if let Some(result) = worker.poll() {
            result.unwrap();
            break;
        }
        assert!(Instant::now() < deadline, "真实本机朗读验证超时");
        thread::sleep(Duration::from_millis(50));
    }
}
