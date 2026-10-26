//! macOS 草稿资格：共享配置策略之外，明确排除私密输入与登录窗口。

use qingjian_predict::live::LiveTranslationConfig;

pub(crate) fn allowed(
    config: &LiveTranslationConfig,
    application: Option<&str>,
    secure: bool,
) -> bool {
    !secure && application != Some("com.apple.loginwindow") && config.draft_allowed_in(application)
}

pub(crate) fn matches(
    config: &LiveTranslationConfig,
    expected: Option<&str>,
    current: Option<&str>,
    secure: bool,
) -> bool {
    expected == current && allowed(config, expected, secure)
}

#[cfg(test)]
mod tests {
    use super::{allowed, matches};
    use qingjian_predict::live::LiveTranslationConfig;

    #[test]
    fn all_known_eligible_apps_share_one_policy_and_unknown_excluded_secure_never_claim_input() {
        let mut config = LiveTranslationConfig {
            enabled: true,
            ..LiveTranslationConfig::default()
        };
        for app in [
            "com.apple.Terminal",
            "com.apple.Safari",
            "com.tencent.xinWeChat",
        ] {
            assert!(allowed(&config, Some(app), false));
            assert!(matches(&config, Some(app), Some(app), false));
            assert!(!allowed(&config, Some(app), true));
        }
        for app in [None, Some(""), Some(" "), Some("com.apple.loginwindow")] {
            assert!(!allowed(&config, app, false));
        }
        assert!(!matches(&config, Some("app.a"), Some("app.b"), false));
        config.draft_excluded_apps.push("app.a".to_owned());
        assert!(!allowed(&config, Some("app.a"), false));
        config.draft_enabled = false;
        assert!(!allowed(&config, Some("app.b"), false));
    }
}
