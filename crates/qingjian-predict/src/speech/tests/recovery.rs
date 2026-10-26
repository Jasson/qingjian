//! 播放失败后旧会话取消、新句使用新 session，避免取消墓碑阻断后续朗读。
use super::http::{accept, configs, reply, request, wait};
use crate::speech::SpeechCoordinator;
use serde_json::json;
use std::net::TcpListener;
use std::thread;

#[test]
fn failed_playback_cancels_old_session_and_next_sentence_can_recover() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let (speech, translation) = configs(&listener);
    let mut worker = SpeechCoordinator::new(speech, translation).unwrap();
    let server = thread::spawn(move || {
        let mut old_session = String::new();
        for failed in [true, false] {
            let (mut stream, _) = accept(&listener);
            let (path, _) = request(&mut stream);
            assert!(path.starts_with("POST /api/generate "));
            reply(&mut stream, 200, json!({"response":"Hello."}));
            let (mut stream, _) = accept(&listener);
            let (path, body) = request(&mut stream);
            assert!(path.starts_with("POST /speak "));
            let session = body["session"].as_str().unwrap().to_owned();
            if failed {
                old_session = session.clone();
            } else {
                assert_ne!(session, old_session);
            }
            reply(&mut stream, 202, json!({"accepted":true}));
            let (mut stream, _) = accept(&listener);
            request(&mut stream);
            reply(
                &mut stream,
                200,
                json!({"state":if failed {"failed"} else {"completed"}}),
            );
            if failed {
                let (mut stream, _) = accept(&listener);
                let (path, body) = request(&mut stream);
                assert!(path.starts_with("POST /cancel "));
                assert_eq!(body["session"], old_session);
                reply(&mut stream, 200, json!({"cancelled":true}));
            }
        }
    });
    worker.update("你好。").unwrap();
    assert!(wait(&mut worker).is_err());
    worker.update("你好。再见。").unwrap();
    wait(&mut worker).unwrap();
    server.join().unwrap();
}
