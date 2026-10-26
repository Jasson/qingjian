//! 纯前台身份核验：未知身份、无效 PID 或私密输入均不创建按键入口。

pub(super) fn matching_pid(
    configured: bool,
    expected: Option<&str>,
    foreground: Option<(&str, i32)>,
    secure: bool,
) -> Option<i32> {
    if !configured || secure {
        return None;
    }
    let expected = expected.filter(|application| !application.trim().is_empty())?;
    let (application, pid) = foreground?;
    (expected == application && pid > 0).then_some(pid)
}

#[cfg(test)]
mod tests {
    use super::matching_pid;

    #[test]
    fn every_known_matching_application_uses_its_own_positive_pid() {
        for (app, pid) in [
            ("com.apple.Terminal", 12),
            ("com.apple.Safari", 34),
            ("com.tencent.xinWeChat", 56),
        ] {
            assert_eq!(
                matching_pid(true, Some(app), Some((app, pid)), false),
                Some(pid)
            );
            assert_eq!(matching_pid(true, Some(app), Some((app, pid)), true), None);
            assert_eq!(
                matching_pid(false, Some(app), Some((app, pid)), false),
                None
            );
        }
    }

    #[test]
    fn unknown_changed_identity_or_invalid_pid_cannot_claim_a_tap() {
        for expected in [None, Some(""), Some(" "), Some("other.app")] {
            assert_eq!(matching_pid(true, expected, Some(("app", 9)), false), None);
        }
        assert_eq!(matching_pid(true, Some("app"), None, false), None);
        for pid in [0, -1, i32::MIN] {
            assert_eq!(
                matching_pid(true, Some("app"), Some(("app", pid)), false),
                None
            );
        }
    }
}
