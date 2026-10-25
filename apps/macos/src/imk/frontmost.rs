//! 前台应用身份读取只在 Host／路由状态借用之外调用。

use objc2_app_kit::NSWorkspace;

pub(crate) fn identity() -> Option<(String, i32)> {
    let application = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    let bundle = application.bundleIdentifier()?.to_string();
    if bundle.trim().is_empty() || bundle == "com.apple.loginwindow" {
        return None;
    }
    Some((bundle, application.processIdentifier()))
}

pub(crate) fn matches(expected: Option<&str>) -> bool {
    identity().is_some_and(|(bundle, _)| Some(bundle.as_str()) == expected)
}
