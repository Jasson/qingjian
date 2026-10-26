//! 请求取消的响应队列回归验证，不发起网络连接。

use std::sync::mpsc;

use crate::live::{LiveTranslator, Translation};

#[test]
fn cancellation_rejects_queued_and_late_replies_then_accepts_new_input() {
    let (requests, incoming) = mpsc::channel();
    let (outgoing, responses) = mpsc::channel();
    let mut translator = LiveTranslator {
        requests,
        responses,
        latest: 0,
    };
    let old = translator.submit("旧句");
    assert_eq!(incoming.recv().unwrap(), Some((old, "旧句".into())));
    let stale = || Translation {
        id: old,
        source: "旧句".into(),
        result: Ok("stale".into()),
    };
    outgoing.send(stale()).unwrap();
    translator.cancel();
    assert_eq!(incoming.recv().unwrap(), None);
    assert!(translator.poll().is_none());
    outgoing.send(stale()).unwrap();
    assert!(translator.poll().is_none());

    let current = translator.submit("新句");
    assert_eq!(incoming.recv().unwrap(), Some((current, "新句".into())));
    outgoing.send(stale()).unwrap();
    outgoing
        .send(Translation {
            id: current,
            source: "新句".into(),
            result: Ok("new".into()),
        })
        .unwrap();
    let result = translator.poll().unwrap();
    assert_eq!(result.id, current);
    assert_eq!(result.result.unwrap(), "new");
}
