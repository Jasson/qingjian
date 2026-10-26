//! 草稿粘贴录键与既有输出键冲突回归。

use super::{Config, Held, Shortcut, press};
use bamboo_slip::{KeyCode, KeyPress};

#[test]
fn paste_recorder_accepts_command_v_and_can_disable() {
    let config = Config::default();
    assert_eq!(
        Shortcut::PasteDraft.config_key(),
        ("shortcut", "paste_draft")
    );
    assert_eq!(
        Shortcut::PasteDraft.accept(
            &press(
                "V",
                Held {
                    logo: true,
                    ..Held::default()
                }
            ),
            &config
        ),
        Ok("command+v".to_owned())
    );
    assert_eq!(
        Shortcut::PasteDraft.accept(&KeyPress::new(KeyCode::Backspace, Held::default()), &config),
        Ok(String::new())
    );
    assert!(
        Shortcut::PasteDraft
            .accept(&press("v", Held::default()), &config)
            .is_err()
    );
    assert!(
        Shortcut::PasteDraft
            .accept(
                &press(
                    "1",
                    Held {
                        logo: true,
                        ..Held::default()
                    }
                ),
                &config
            )
            .is_err()
    );
}

#[test]
fn paste_conflicts_are_rejected_without_changing_translation_output() {
    let mut config = Config::default();
    config.shortcut.insert_live_translation = Some("command+shift+y".parse().unwrap());
    let custom = Held {
        logo: true,
        shift: true,
        ..Held::default()
    };
    assert!(
        Shortcut::PasteDraft
            .accept(&press("y", custom), &config)
            .is_err()
    );
    let existing = Held {
        logo: true,
        ..Held::default()
    };
    for action in [
        Shortcut::InsertLiveTranslation,
        Shortcut::InsertOriginal,
        Shortcut::OpenTranslationDraft,
    ] {
        assert!(action.accept(&press("v", existing), &config).is_err());
    }
    assert!(
        Shortcut::PasteDraft
            .accept(
                &press(
                    "e",
                    Held {
                        control: true,
                        alt: true,
                        ..Held::default()
                    }
                ),
                &config
            )
            .is_err()
    );
    assert_eq!(
        config.shortcut.insert_live_translation,
        Some("command+shift+y".parse().unwrap())
    );
}
