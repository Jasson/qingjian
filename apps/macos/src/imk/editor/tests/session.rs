//! 同一个词库下的普通 IMK 与草稿输入会话必须互不覆盖，异常展开也恢复原状态。

use std::panic::{AssertUnwindSafe, catch_unwind};

use qingjian_core::EngineSession;
use qingjian_predict::live::FocusedDraftModel;

use super::engine;
use crate::imk::editor::session::SessionLease;

#[test]
fn lease_restores_imk_composition_mode_history_and_application() {
    let mut engine = engine();
    engine.set_input("shijie");
    engine.set_application(Some("com.tencent.xinWeChat".into()));
    engine.history_mut().record("普通前文");
    let mut parked = EngineSession::default();
    let mut draft = FocusedDraftModel::new();
    {
        let mut lease = SessionLease::new(&mut engine, &mut parked);
        assert!(lease.is_private());
        assert!(lease.composition().is_empty());
        assert!(lease.history().is_empty());
        assert_eq!(lease.application(), None);
        draft.text(&mut lease, "nihao", false, true);
        assert_eq!(draft.source(), "你好");
        assert!(draft.choose(&mut lease, 0));
        draft.text(&mut lease, "English", true, true);
        assert_eq!(draft.committed(), "你好English");
        assert!(lease.english_mode());
    }
    assert!(!engine.is_private());
    assert!(!engine.english_mode());
    assert_eq!(engine.composition().text(), "shijie");
    assert_eq!(engine.history().text(), "普通前文");
    assert_eq!(engine.application(), Some("com.tencent.xinWeChat"));
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "世界");
    {
        let mut lease = SessionLease::new(&mut engine, &mut parked);
        assert!(lease.english_mode());
        assert!(lease.composition().is_empty());
        draft.text(&mut lease, "ni", false, true);
        assert_eq!(draft.raw_keys(), "ni");
    }
    assert_eq!(engine.composition().text(), "shijie");
    assert_eq!(draft.committed(), "你好English");
}

#[test]
fn unwind_restores_private_switch_and_both_live_compositions() {
    for private in [false, true] {
        let mut engine = engine();
        engine.set_private(private);
        engine.set_input("shijie");
        let mut parked = EngineSession::default();
        let failure = catch_unwind(AssertUnwindSafe(|| {
            let mut lease = SessionLease::new(&mut engine, &mut parked);
            lease.set_input("nihao");
            panic!("测试独立会话异常退出");
        }));
        assert!(failure.is_err());
        assert_eq!(engine.is_private(), private);
        assert_eq!(engine.composition().text(), "shijie");
        {
            let lease = SessionLease::new(&mut engine, &mut parked);
            assert_eq!(lease.composition().text(), "nihao");
            assert!(lease.is_private());
            assert_eq!(lease.query().unwrap().candidates.items[0].text, "你好");
        }
        assert_eq!(engine.is_private(), private);
        assert_eq!(engine.composition().text(), "shijie");
    }
}
