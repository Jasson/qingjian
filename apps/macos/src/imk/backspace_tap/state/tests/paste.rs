//! 草稿粘贴复用真实事件队列：精确组合、空态透传、重复隔离和冲突消费。

use super::{active, event};
use crate::imk::backspace_tap::state::State;
use crate::imk::backspace_tap::{Action, Bindings};
use qingjian_platform::{DraftShortcut, ShortcutConfig};

fn prepared() -> State<u64> {
    let mut state = active();
    state.publish_bindings(Bindings::for_config(&ShortcutConfig::default()));
    state
}

#[test]
fn command_v_is_exact_and_empty_draft_passes_to_application() {
    let mut state = prepared();
    assert!(state.wants_imk_action(9, 1 << 20, false));
    for flags in [
        0,
        1 << 17,
        (1 << 20) | (1 << 17),
        (1 << 20) | (1 << 19),
        (1 << 20) | (1 << 18),
    ] {
        assert!(!state.wants_imk_action(9, flags, false));
        let mut key = event(&state, 10, 9, false);
        key.flags = flags;
        assert!(!state.receive(key));
    }
    state.publish(2, true, false);
    assert!(!state.wants_imk_action(9, 1 << 20, false));
    let mut key = event(&state, 10, 9, false);
    key.flags = 1 << 20;
    assert!(!state.receive(key));
    assert!(state.batch().is_none());
}

#[test]
fn paste_is_queued_once_and_consumption_cancels_old_output_and_repeat() {
    let mut state = prepared();
    let mut key = event(&state, 10, 9, false);
    key.flags = 1 << 20;
    assert!(state.receive(key));
    assert!(state.owns_imk_key(key.timestamp, 9));
    for _ in 0..10 {
        key.repeat = true;
        assert!(state.receive(key));
    }
    let batch = state.batch().unwrap();
    assert_eq!(batch.action, Action::PasteDraft);
    assert_eq!(batch.count, 1);
    assert!(state.receive(event(&state, 10, 36, false)));
    // 成功和拒绝均消费同一个意图；旧回车不能输出刚粘贴的新稿。
    state.action_consumed(batch.generation, batch.serial);
    state.publish(2, true, true);
    assert!(state.batch().is_none());
    assert!(state.receive(key));
    assert!(state.batch().is_none());
    assert!(state.receive(event(&state, 11, 9, false)));
    key.repeat = false;
    assert!(state.receive(key));
    assert_eq!(state.batch().unwrap().action, Action::PasteDraft);
}

#[test]
fn paste_conflicts_consume_the_binding_without_output_and_keep_other_keys() {
    for other in 0..3 {
        let mut config = ShortcutConfig::default();
        match other {
            0 => config.insert_live_translation = config.paste_draft,
            1 => config.insert_original = config.paste_draft.map(DraftShortcut::Combo),
            _ => config.open_translation_draft = config.paste_draft,
        }
        let mut state = prepared();
        state.publish_bindings(Bindings::for_config(&config));
        assert!(config.paste_draft_conflicts());
        let mut key = event(&state, 10, 9, false);
        key.flags = 1 << 20;
        assert!(state.receive(key));
        assert_eq!(state.batch().unwrap().action, Action::PasteConflict);
    }
    let config = ShortcutConfig {
        insert_live_translation: Some("command+shift+y".parse().unwrap()),
        ..ShortcutConfig::default()
    };
    assert!(!config.paste_draft_conflicts());
    assert_eq!(
        config.insert_live_translation.unwrap().to_string(),
        "shift+command+y"
    );
    let mut state = prepared();
    state.publish_bindings(Bindings::for_config(&config).without_paste());
    assert!(!state.wants_imk_action(9, 1 << 20, false));
    assert!(state.wants_imk_action(36, 0, false));
}

#[test]
fn custom_or_disabled_paste_and_secure_events_never_fall_back_to_default() {
    let mut config = ShortcutConfig {
        paste_draft: Some("control+option+p".parse().unwrap()),
        ..ShortcutConfig::default()
    };
    let mut state = prepared();
    state.publish_bindings(Bindings::for_config(&config));
    assert!(!state.wants_imk_action(9, 1 << 20, false));
    assert!(state.wants_imk_action(35, (1 << 18) | (1 << 19), false));
    config.paste_draft = None;
    state.publish_bindings(Bindings::for_config(&config));
    assert!(!state.wants_imk_action(9, 1 << 20, false));
    let mut state = prepared();
    let mut key = event(&state, 10, 9, false);
    key.flags = 1 << 20;
    key.secure = true;
    assert!(!state.receive(key));
    assert!(state.batch().is_none());
}
