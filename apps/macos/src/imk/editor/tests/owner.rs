//! 生产 IMK 路由门禁与真实 Core 会话联合回放，拒绝旧输出闭包后仍保留两份独立输入。

use qingjian_core::EngineSession;
use qingjian_predict::live::FocusedDraftModel;

use super::{engine, scope::OwnerScope};
use crate::imk::editor::opening_guard::OpeningGuard;
use crate::imk::editor::{owner::Owner, session::SessionLease};

#[test]
fn opening_visible_and_pending_never_run_the_legacy_imk_output() {
    let _scope = OwnerScope::enter(Owner::Idle);
    let mut engine = engine();
    engine.set_input("shijie");
    let mut session = EngineSession::default();
    let mut editor = FocusedDraftModel::new();
    let mut output_calls = 0;
    for phase in [
        Owner::Opening(123),
        Owner::Visible(123),
        Owner::Pending(123),
    ] {
        phase.set("test_workflow");
        assert!(
            Owner::legacy_route(|| {
                output_calls += 1;
                engine.take_raw()
            })
            .is_none()
        );
        assert_eq!(engine.composition().text(), "shijie");
        {
            let mut lease = SessionLease::new(&mut engine, &mut session);
            if editor.raw_keys().is_empty() {
                editor.text(&mut lease, "nihao", false, true);
            }
            assert_eq!(editor.source(), "你好");
        }
        assert_eq!(engine.composition().text(), "shijie");
    }
    assert_eq!(output_calls, 0);
    Owner::Idle.set("test_close");
    assert_eq!(
        Owner::legacy_route(|| {
            output_calls += 1;
            engine.take_raw()
        })
        .as_deref(),
        Some("shijie")
    );
    assert_eq!(output_calls, 1);
    assert_eq!(editor.raw_keys(), "nihao");
    assert_eq!(editor.source(), "你好");
}

#[test]
fn releasing_control_allows_terminal_input_without_consuming_the_hidden_editor() {
    let _scope = OwnerScope::enter(Owner::Visible(123));
    let mut engine = engine();
    let mut session = EngineSession::default();
    let mut editor = FocusedDraftModel::new();
    {
        let mut lease = SessionLease::new(&mut engine, &mut session);
        editor.text(&mut lease, "nihao", false, true);
    }
    Owner::Idle.set("test_hidden_for_terminal");
    let mut terminal = FocusedDraftModel::new();
    assert_eq!(
        Owner::legacy_route(|| terminal.text(&mut engine, "echo hello", true, true)),
        Some(true)
    );
    assert_eq!(terminal.original_output().as_deref(), Some("echo hello"));
    assert_eq!(editor.raw_keys(), "nihao");
    Owner::Visible(123).set("test_resume_editor");
    {
        let lease = SessionLease::new(&mut engine, &mut session);
        assert_eq!(lease.composition().text(), "nihao");
        assert_eq!(lease.query().unwrap().candidates.items[0].text, "你好");
    }
    assert_eq!(terminal.original_output().as_deref(), Some("echo hello"));
}

#[test]
fn production_opening_guard_restores_legacy_input_after_failure_or_unwind() {
    let _scope = OwnerScope::enter(Owner::Idle);
    let mut engine = engine();
    engine.set_input("shijie");
    for unwind in [false, true] {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Owner::Opening(123).set("test_open_begin");
            let _rollback = OpeningGuard::new();
            assert!(Owner::legacy_route(|| engine.take_raw()).is_none());
            if unwind {
                panic!("模拟开窗准备失败");
            }
        }));
        assert_eq!(result.is_err(), unwind);
        assert_eq!(Owner::current(), Owner::Idle);
        assert_eq!(engine.composition().text(), "shijie");
        assert_eq!(
            Owner::legacy_route(|| engine.query().unwrap().candidates.items[0].text.clone()),
            Some("世界".into())
        );
    }
}

#[test]
fn stale_opening_guard_cannot_rollback_a_new_same_pid_workflow() {
    let _scope = OwnerScope::enter(Owner::Idle);
    for next in [
        Owner::Opening(123),
        Owner::Visible(123),
        Owner::Pending(123),
        Owner::Idle,
    ] {
        Owner::Opening(123).set("test_old_open");
        let old_ticket = Owner::generation();
        let stale = OpeningGuard::new();
        Owner::Idle.set("test_cancel_or_focus_lost");
        next.set("test_new_same_pid_workflow");
        let new_ticket = Owner::generation();
        assert_ne!(old_ticket, new_ticket);
        drop(stale);
        assert_eq!(Owner::current(), next);
        assert_eq!(Owner::generation(), new_ticket);
    }
}

#[test]
fn cancelled_opening_guard_cannot_reclaim_control_after_secure_or_config_cancel() {
    let _scope = OwnerScope::enter(Owner::Idle);
    Owner::Opening(123).set("test_open_before_cancel");
    let guard = OpeningGuard::new();
    Owner::Idle.set("test_secure_or_config_cancel");
    let cancelled_ticket = Owner::generation();
    drop(guard);
    assert_eq!(Owner::current(), Owner::Idle);
    assert_eq!(Owner::generation(), cancelled_ticket);
    assert_eq!(Owner::legacy_route(|| "正常输入"), Some("正常输入"));
}
