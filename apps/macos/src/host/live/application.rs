//! 宿主文档能力与选区替换的应用身份边界。

use qingjian_predict::live::context::ContextPolicy;

pub(crate) fn context_policy(current: Option<&str>, last_known: Option<&str>) -> ContextPolicy {
    if current.or(last_known) == Some("com.apple.Terminal") {
        ContextPolicy::EventOnly
    } else {
        ContextPolicy::VerifiedDocument
    }
}

pub(super) fn replacement_application_blocker(
    current: Option<&str>,
    last_known: Option<&str>,
) -> Option<&'static str> {
    if current == Some("com.apple.Terminal") || last_known == Some("com.apple.Terminal") {
        Some("terminal_readonly_selection")
    } else if current.is_none() && last_known.is_none() {
        Some("application_unknown")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{context_policy, replacement_application_blocker};
    use qingjian_predict::live::context::ContextPolicy;

    #[test]
    fn terminal_event_policy_survives_missing_identity_and_leaves_on_known_app_change() {
        assert_eq!(
            context_policy(Some("com.apple.Terminal"), None),
            ContextPolicy::EventOnly
        );
        assert_eq!(
            context_policy(None, Some("com.apple.Terminal")),
            ContextPolicy::EventOnly
        );
        assert_eq!(
            context_policy(Some("test.browser"), Some("com.apple.Terminal")),
            ContextPolicy::VerifiedDocument
        );
        assert_eq!(context_policy(None, None), ContextPolicy::VerifiedDocument);
    }

    #[test]
    fn terminal_and_missing_identity_never_authorize_selected_text_write() {
        assert_eq!(
            replacement_application_blocker(Some("com.apple.Terminal"), None),
            Some("terminal_readonly_selection")
        );
        assert_eq!(
            replacement_application_blocker(None, Some("com.apple.Terminal")),
            Some("terminal_readonly_selection")
        );
        assert_eq!(
            replacement_application_blocker(None, None),
            Some("application_unknown")
        );
        assert_eq!(
            replacement_application_blocker(None, Some("test.editor")),
            None
        );
        assert_eq!(
            replacement_application_blocker(Some("test.editor"), None),
            None
        );
    }
}
