//! 自动翻译的首段输入资格；只判断启动与空态直通，不改变已有原文或手动草稿。

mod source;
pub use source::AutomaticSource;

/// 字母或汉字可启动自动翻译；独立 emoji 字形不能当作字母启动。
pub fn starts_automatic_source(text: &str) -> bool {
    text.chars().any(|ch| {
        let mut encoded = [0; 4];
        // 扩展汉字不属于 emoji；Core 的候选筛选将高平面字符合并识别，需保留这些汉字。
        ch.is_alphabetic()
            && !ch.is_numeric()
            && (u32::from(ch) > 0x1FFFF
                || !qingjian_core::emoji::is_emoji(ch.encode_utf8(&mut encoded)))
    })
}

/// 真空态非语言输入交给宿主；Core 问字入口与已有组句／正文／弹层继续走原路径。
pub fn passes_automatic_empty(
    text: Option<&str>,
    source: &str,
    composing: bool,
    occupied: bool,
    question_mark: bool,
) -> bool {
    let Some(text) = text.filter(|text| !text.is_empty()) else {
        return false;
    };
    if !source.is_empty()
        || composing
        || occupied
        || question_mark && text == "?"
        || text.chars().any(char::is_control)
    {
        return false;
    }
    !starts_automatic_source(text)
}

/// 已启动的内容即使删成数字或空格也保留；恢复能力与提示不构成正文。
pub fn automatic_translation_visible(source: &str, draft: &str, composing: bool) -> bool {
    composing || !source.is_empty() || !draft.is_empty()
}

#[cfg(test)]
mod tests;
