//! 本机 HTTP 回放：确认多句顺序、播放终态、取消和无历史重播。
use crate::live::LiveTranslationConfig;
use crate::speech::{SpeechConfig, SpeechCoordinator};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub(super) fn accept(listener: &TcpListener) -> (TcpStream, std::net::SocketAddr) {
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok(stream) => {
                stream.0.set_nonblocking(false).unwrap();
                return stream;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "未收到预期HTTP请求");
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("{error}"),
        }
    }
}
pub(super) fn request(stream: &mut TcpStream) -> (String, Value) {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut bytes = Vec::new();
    loop {
        let mut buffer = [0; 4096];
        let n = stream.read(&mut buffer).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&buffer[..n]);
        let raw = String::from_utf8_lossy(&bytes);
        if let Some((header, body)) = raw.split_once("\r\n\r\n") {
            let len = header
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            if body.len() >= len {
                return (
                    header.lines().next().unwrap().into(),
                    if len == 0 {
                        Value::Null
                    } else {
                        serde_json::from_str(&body[..len]).unwrap()
                    },
                );
            }
        }
    }
}
pub(super) fn reply(stream: &mut TcpStream, code: u16, body: Value) {
    let body = body.to_string();
    let _ = write!(
        stream,
        "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}
pub(super) fn configs(listener: &TcpListener) -> (SpeechConfig, LiveTranslationConfig) {
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    (
        SpeechConfig {
            base_url: base_url.clone(),
            ..Default::default()
        },
        LiveTranslationConfig {
            base_url,
            ..Default::default()
        },
    )
}
pub(super) fn wait(worker: &mut SpeechCoordinator) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        if let Some(reply) = worker.poll() {
            return reply;
        }
        assert!(Instant::now() < deadline, "未收到播放终态");
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn translates_each_confirmed_sentence_in_order_and_waits_for_completion() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let (speech, translation) = configs(&listener);
    let mut worker = SpeechCoordinator::new(speech, translation).unwrap();
    let server = thread::spawn(move || {
        let mut ids = Vec::new();
        for sentence in ["你好。", "世界！"] {
            let (mut stream, _) = accept(&listener);
            let (path, body) = request(&mut stream);
            assert!(path.starts_with("POST /api/generate "));
            assert!(body["prompt"].as_str().unwrap().ends_with(sentence));
            reply(
                &mut stream,
                200,
                json!({"response":format!("translated {sentence}")}),
            );
            let (mut stream, _) = accept(&listener);
            let (path, body) = request(&mut stream);
            assert!(path.starts_with("POST /speak "));
            assert_eq!(body["text"], format!("translated {sentence}"));
            ids.push(body["id"].as_str().unwrap().to_owned());
            reply(&mut stream, 202, json!({"accepted":true,"state":"queued"}));
            let (mut stream, _) = accept(&listener);
            let (path, _) = request(&mut stream);
            assert!(path.starts_with("GET /status?"));
            reply(&mut stream, 200, json!({"state":"completed"}));
        }
        assert_ne!(ids[0], ids[1]);
    });
    worker.update("你好。世界！未完成").unwrap();
    worker.update("你好。世界！未完成").unwrap();
    assert!(worker.busy());
    wait(&mut worker).unwrap();
    wait(&mut worker).unwrap();
    assert!(!worker.busy());
    server.join().unwrap();
}
#[test]
fn cancel_during_translation_never_speaks_late_reply() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let (speech, translation) = configs(&listener);
    let mut worker = SpeechCoordinator::new(speech, translation).unwrap();
    let (started, received) = mpsc::channel();
    let server = thread::spawn(move || {
        let (mut old, _) = accept(&listener);
        request(&mut old);
        started.send(()).unwrap();
        let (mut cancel, _) = accept(&listener);
        let (path, body) = request(&mut cancel);
        assert!(path.starts_with("POST /cancel "));
        assert!(body["session"].is_string());
        reply(&mut cancel, 200, json!({"cancelled":true}));
        reply(&mut old, 200, json!({"response":"late translation"}));
    });
    worker.update("你好。").unwrap();
    received.recv_timeout(Duration::from_secs(3)).unwrap();
    worker.cancel();
    server.join().unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while worker.busy() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    assert!(worker.poll().is_none());
}
#[test]
fn rejects_nonlocal_service_and_invalid_speed() {
    let live = LiveTranslationConfig::default();
    assert!(
        SpeechCoordinator::new(
            SpeechConfig {
                base_url: "http://example.com".into(),
                ..Default::default()
            },
            live.clone()
        )
        .is_err()
    );
    assert!(
        SpeechCoordinator::new(
            SpeechConfig {
                speed: f64::NAN,
                ..Default::default()
            },
            live
        )
        .is_err()
    );
}
#[test]
fn baseline_is_not_automatically_spoken_and_queue_is_bounded() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let (speech, translation) = configs(&listener);
    let mut worker = SpeechCoordinator::new(speech, translation).unwrap();
    worker.baseline("历史。");
    worker.update("历史。").unwrap();
    assert!(!worker.busy());
    assert!(
        worker
            .update(&format!("历史。{}", "新句。".repeat(40)))
            .is_err()
    );
}
