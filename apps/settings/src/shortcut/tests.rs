//! 快捷键录制校验与拒绝保存回归。

use super::*;
use bamboo_slip::Modifiers as Held;

fn press(key: &str, held: Held) -> KeyPress {
    KeyPress::new(KeyCode::Character(key.to_owned()), held)
}

fn alt() -> Held {
    Held {
        alt: true,
        ..Held::default()
    }
}

#[test]
fn modifier_sets_must_be_held_and_distinct() {
    let config = Config::default();
    let (first, _) = config.shortcut.translation_keys();
    assert!(
        Shortcut::Translation
            .accept(&press("1", Held::default()), &config)
            .is_err()
    );
    // 删除候选不能跟第一组译词键一样
    let same = Held {
        alt: first.option,
        shift: first.shift,
        control: first.control,
        logo: first.command,
        ..Held::default()
    };
    let clash = Shortcut::DeleteCandidate.accept(&press("1", same), &config);
    assert_eq!(clash, Err("跟「上屏第一个译词」重复了".to_owned()));
    // 自己跟自己不算撞
    assert_eq!(
        Shortcut::Translation.accept(&press("1", same), &config),
        Ok(first.key())
    );
}

#[test]
fn translate_selection_needs_a_letter_and_aux_key_a_symbol() {
    let config = Config::default();
    assert_eq!(
        Shortcut::TranslateSelection.accept(&press("T", alt()), &config),
        Ok("option+t".to_owned())
    );
    assert!(
        Shortcut::TranslateSelection
            .accept(&press("t", Held::default()), &config)
            .is_err()
    );
    assert_eq!(
        Shortcut::AuxCodeKey.accept(&press("/", Held::default()), &config),
        Ok("/".to_owned())
    );
    assert!(
        Shortcut::AuxCodeKey
            .accept(&press("a", Held::default()), &config)
            .is_err()
    );
    assert!(
        Shortcut::AuxCodeKey
            .accept(&press("[", Held::default()), &config)
            .is_err()
    );
    assert!(
        Shortcut::AuxCodeKey
            .accept(&press(";", alt()), &config)
            .is_err()
    );
}

#[test]
fn live_actions_can_be_disabled_and_reject_conflicting_keys() {
    let config = Config::default();
    for shortcut in [
        Shortcut::CopyLiveTranslation,
        Shortcut::ReplaceLiveTranslation,
        Shortcut::InsertLiveTranslation,
    ] {
        assert_eq!(
            shortcut.accept(&KeyPress::new(KeyCode::Backspace, Held::default()), &config),
            Ok(String::new())
        );
        assert!(
            shortcut
                .accept(&press("c", Held::default()), &config)
                .is_err()
        );
    }
    let held = Held {
        control: true,
        alt: true,
        ..Held::default()
    };
    assert!(
        Shortcut::CopyLiveTranslation
            .accept(&press("r", held), &config)
            .is_err()
    );
    assert!(
        Shortcut::ReplaceLiveTranslation
            .accept(&press("t", held), &config)
            .is_err()
    );
    assert_eq!(
        Shortcut::TranslateSelection
            .accept(&press("c", held), &config)
            .is_err(),
        crate::support::Feature::LiveTranslation.here(),
    );
    assert!(
        Shortcut::InsertLiveTranslation
            .accept(&press("r", held), &config)
            .is_err()
    );
    assert!(
        Shortcut::ReplaceLiveTranslation
            .accept(&press("v", held), &config)
            .is_err()
    );
    assert_eq!(
        Shortcut::InsertLiveTranslation.accept(&press("V", held), &config),
        Ok("control+option+v".to_owned())
    );
    assert_eq!(
        Shortcut::CopyLiveTranslation.accept(&press("C", held), &config),
        Ok("control+option+c".to_owned())
    );
    assert_eq!(
        Shortcut::ReplaceLiveTranslation.accept(&press("x", held), &config),
        Ok("control+option+x".to_owned())
    );
}

#[test]
fn invalid_live_recording_keeps_saved_shortcut() {
    use crate::message::Message;
    use crate::settings::Settings;
    use bamboo_slip::App;

    let directory =
        std::env::temp_dir().join(format!("qingjian-live-recording-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("config.toml");
    Config::set_value(
        &path,
        "shortcut",
        "copy_live_translation",
        "control+option+x",
    )
    .unwrap();
    let mut settings = Settings::load(path.clone()).unwrap();
    let before = std::fs::read_to_string(&path).unwrap();
    for shortcut in [
        Shortcut::CopyLiveTranslation,
        Shortcut::ReplaceLiveTranslation,
        Shortcut::InsertLiveTranslation,
        Shortcut::InsertOriginal,
        Shortcut::CancelDraft,
    ] {
        for key in ["1", "9", ".", "中"] {
            settings.update(Message::Recorded(shortcut, press(key, alt())));
            assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
            assert_eq!(settings.config, Config::load(&path).unwrap());
            assert!(settings.notice.is_some());
        }
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn prediction_recorder_accepts_modified_letters_and_period() {
    let config = Config::default();
    assert_eq!(Shortcut::Predict.config_key(), ("shortcut", "predict"));
    assert_eq!(
        Shortcut::Predict.accept(&press("J", alt()), &config),
        Ok("option+j".to_owned())
    );
    assert_eq!(
        Shortcut::Predict.accept(&press(".", alt()), &config),
        Ok("option+.".to_owned())
    );
    assert!(
        Shortcut::Predict
            .accept(&press("j", Held::default()), &config)
            .is_err()
    );
}

#[test]
fn draft_action_recording_accepts_special_keys_and_rejects_conflicts() {
    let mut config = Config::default();
    assert_eq!(
        Shortcut::InsertOriginal.accept(&KeyPress::new(KeyCode::Enter, Held::default()), &config),
        Ok("enter".to_owned())
    );
    assert_eq!(
        Shortcut::CancelDraft.accept(&KeyPress::new(KeyCode::Escape, Held::default()), &config),
        Ok("escape".to_owned())
    );
    assert!(
        Shortcut::InsertOriginal
            .accept(&KeyPress::new(KeyCode::Escape, Held::default()), &config)
            .is_err()
    );
    assert!(
        Shortcut::CancelDraft
            .accept(&KeyPress::new(KeyCode::Enter, Held::default()), &config)
            .is_err()
    );
    for action in [Shortcut::InsertOriginal, Shortcut::CancelDraft] {
        assert_eq!(
            action.accept(&KeyPress::new(KeyCode::Backspace, Held::default()), &config),
            Ok(String::new())
        );
        for invalid in ["a", "1", "."] {
            assert!(
                action
                    .accept(&press(invalid, Held::default()), &config)
                    .is_err()
            );
        }
        let held = Held {
            control: true,
            alt: true,
            ..Held::default()
        };
        for key in ["c", "r", "v", "t", "j"] {
            assert!(action.accept(&press(key, held), &config).is_err());
        }
    }
    config.shortcut.cancel_draft = Some(DraftShortcut::Combo("option+x".parse().unwrap()));
    assert!(
        Shortcut::InsertOriginal
            .accept(&press("x", alt()), &config)
            .is_err()
    );
    assert!(
        Shortcut::InsertLiveTranslation
            .accept(&press("x", alt()), &config)
            .is_err()
    );
    assert_eq!(
        Shortcut::InsertOriginal.accept(&press("z", alt()), &config),
        Ok("option+z".to_owned())
    );
}
