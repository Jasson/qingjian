//! 交错记录预算、重复抑制与普通按键隐私分类回归。

use super::Limiter;
use crate::imk::controller::entry_diagnostics::kind::Kind;
use std::time::Duration;

#[test]
fn interleaved_categories_cannot_exceed_sixteen_including_activation() {
    let mut limiter = Limiter::default();
    limiter.arm(1);
    let mut records = 1; // arm marker 已占总预算。
    for second in 0..100 {
        for kind in [
            Kind::NonDeleteKeyDown,
            Kind::BackspaceKey,
            Kind::MissingClient,
            Kind::ForwardDeleteKey,
            Kind::MissingEvent,
            Kind::BackspaceControl,
            Kind::DeleteControl,
        ] {
            if limiter.allow(kind, Duration::from_secs(second), false) {
                records += 1;
            }
        }
    }
    assert_eq!(records, 16);
    assert_eq!(limiter.ordinary_count(), 100);
}

#[test]
fn held_repeats_and_cooldown_do_not_spend_special_budget() {
    let mut limiter = Limiter::default();
    limiter.arm(1);
    assert!(limiter.allow(Kind::BackspaceKey, Duration::ZERO, false));
    for second in 0..100 {
        assert!(!limiter.allow(Kind::BackspaceKey, Duration::from_secs(second), true));
    }
    assert!(!limiter.allow(Kind::BackspaceKey, Duration::from_millis(500), false));
    for second in 1..12 {
        assert!(limiter.allow(Kind::BackspaceKey, Duration::from_secs(second), false));
    }
    assert!(!limiter.allow(Kind::BackspaceKey, Duration::from_secs(12), false));
}

#[test]
fn late_first_press_and_first_repeat_are_visible_without_a_time_window_or_stage_gate() {
    let mut limiter = Limiter::default();
    limiter.arm(1);
    let late = Duration::from_secs(600);
    // 即使首个观测事件已带 repeat，也保留一次证据，随后长按不刷记录。
    assert!(limiter.allow(Kind::BackspaceKey, late, true));
    assert!(!limiter.allow(Kind::BackspaceKey, late + Duration::from_secs(2), true));
    // staged、Host 可用性均不是限流/选择条件；nil 入口各有独立预算。
    assert!(limiter.allow(Kind::MissingClient, late, false));
    assert!(limiter.allow(Kind::MissingEvent, late, false));
    assert!(!limiter.allow(Kind::MissingClient, late + Duration::from_secs(10), false));
}

#[test]
fn reactivation_resets_budget_and_late_deactivation_cannot_disarm_new_session() {
    let mut limiter = Limiter::default();
    assert!(!limiter.allow(Kind::BackspaceKey, Duration::ZERO, false));
    let first = limiter.arm(1);
    assert!(limiter.allow(Kind::BackspaceKey, Duration::ZERO, false));
    let second = limiter.arm(2);
    assert!(second > first);
    assert!(!limiter.armed_for(1));
    assert!(limiter.armed_for(2));
    limiter.disarm(1);
    assert!(limiter.armed());
    assert!(limiter.allow(Kind::BackspaceKey, Duration::ZERO, false));
    limiter.disarm(2);
    assert!(!limiter.allow(Kind::BackspaceKey, Duration::from_secs(2), false));
}

#[test]
fn ordinary_keys_collapse_to_one_heartbeat_without_loggable_key_codes() {
    let mut limiter = Limiter::default();
    limiter.arm(1);
    let mut heartbeat = 0;
    for key in [0, 11, 8, 2, 14, 3, 5, 4, 34, 38, 40, 37] {
        let kind = Kind::classify(true, true, true, Some(key), Some(b'a'.into()));
        assert_eq!(kind, Kind::NonDeleteKeyDown);
        assert!(kind.key_code().is_none());
        assert!(kind.special_slot().is_none());
        heartbeat += usize::from(limiter.allow(kind, Duration::ZERO, false));
    }
    assert_eq!(heartbeat, 1);
    assert_eq!(limiter.ordinary_count(), 12);
}

#[test]
fn nil_and_delete_forms_remain_distinct_without_exposing_a_control_h_letter_code() {
    assert_eq!(
        Kind::classify(false, false, false, None, None),
        Kind::MissingEvent
    );
    assert_eq!(
        Kind::classify(true, false, true, Some(51), Some(127)),
        Kind::MissingClient
    );
    assert_eq!(
        Kind::classify(true, true, true, Some(51), Some(127)),
        Kind::BackspaceKey
    );
    assert_eq!(
        Kind::classify(true, true, true, Some(117), None),
        Kind::ForwardDeleteKey
    );
    let ctrl_h = Kind::classify(true, true, true, Some(4), Some(8));
    assert_eq!(ctrl_h, Kind::BackspaceControl);
    assert_eq!(ctrl_h.key_code(), None);
    assert_eq!(
        Kind::classify(true, true, true, Some(0), Some(127)),
        Kind::DeleteControl
    );
    assert_eq!(Kind::classify(true, true, false, None, None), Kind::Other);
}
