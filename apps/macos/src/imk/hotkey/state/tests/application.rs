//! 草稿应用身份、排除项与按键快照的注册门禁。

use super::{State, binding};
use crate::imk::hotkey::binding::Binding;

#[test]
fn unknown_excluded_disabled_or_missing_controller_never_produces_a_registration() {
    use qingjian_predict::live::LiveTranslationConfig;
    let combo = Some(binding('y', 1).combo);
    let config = LiveTranslationConfig {
        enabled: true,
        ..LiveTranslationConfig::default()
    };
    for app in [
        "com.apple.Terminal",
        "com.apple.Safari",
        "com.tencent.xinWeChat",
    ] {
        assert!(Binding::for_session(combo, Some(1), Some(app), &config).is_some());
    }
    for (owner, application, enabled, draft) in [
        (None, Some("com.apple.Terminal"), true, true),
        (Some(0), Some("com.apple.Safari"), true, true),
        (Some(1), Some("excluded.app"), true, true),
        (Some(1), Some("com.apple.loginwindow"), true, true),
        (Some(1), None, true, true),
        (Some(1), Some(""), true, true),
        (Some(1), Some(" "), true, true),
        (Some(1), Some("com.apple.Terminal"), false, true),
        (Some(1), Some("com.apple.Terminal"), true, false),
    ] {
        let config = LiveTranslationConfig {
            enabled,
            draft_enabled: draft,
            draft_excluded_apps: vec!["excluded.app".to_owned()],
            ..LiveTranslationConfig::default()
        };
        let wanted = Binding::for_session(combo, owner, application, &config);
        assert!(wanted.is_none());
        let mut state = State::<u64>::default();
        state.change(wanted);
        assert!(state.registration_request().is_none());
        assert!(!state.needs_monitor());
    }
    assert!(Binding::for_session(None, Some(1), Some("com.apple.Terminal"), &config).is_none());
}

#[test]
fn changed_application_snapshot_or_exclusion_never_dispatches_the_old_press() {
    use qingjian_predict::live::LiveTranslationConfig;
    let mut config = LiveTranslationConfig {
        enabled: true,
        ..LiveTranslationConfig::default()
    };
    for app in [
        "com.apple.Terminal",
        "com.apple.Safari",
        "com.tencent.xinWeChat",
    ] {
        let mut state = State::default();
        let desired =
            Binding::for_session(Some(binding('v', 1).combo), Some(1), Some(app), &config).unwrap();
        state.change(Some(desired));
        let (generation, _) = state.registration_request().unwrap();
        state.complete(generation, Ok(10));
        state.publish(app);
        assert!(state.receive(generation, true));
        state.publish("different.application");
        assert!(state.start_dispatch().is_none());
        state.receive(generation, false);
        state.publish(app);
        assert!(state.receive(generation, true));
        config.draft_excluded_apps.push(app.to_owned());
        state.change(Binding::for_session(
            Some(desired.combo),
            Some(1),
            Some(app),
            &config,
        ));
        assert!(!state.current(generation));
        assert!(!state.owns(desired.combo));
        assert!(state.start_dispatch().is_none());
        assert_eq!(state.retiring(), Some(10));
    }
}
