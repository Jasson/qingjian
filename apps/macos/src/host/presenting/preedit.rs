//! 草稿模式不向宿主发送 marked，因此必须在青简自己的候选窗显示待选按键。

use crate::candidates::Preedit;
use qingjian_platform::PreeditMode;

pub(super) fn candidate_preedit(
    mode: PreeditMode,
    draft: bool,
    preedit: Option<&Preedit>,
) -> Option<Preedit> {
    (mode.in_window() || draft)
        .then(|| preedit.cloned())
        .flatten()
}

/// 显示光标附近的按键；长拼音必须保留末尾，不能退格时一直显示不变的开头。
pub(crate) fn pending_input_line(text: &str, cursor: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let cursor = text
        .char_indices()
        .take_while(|(index, _)| *index < cursor)
        .count();
    let start = cursor.saturating_sub(20);
    let end = (cursor + 8).min(chars.len());
    let before: String = chars[start..cursor].iter().collect();
    let after: String = chars[cursor..end].iter().collect();
    format!(
        "待选拼音：{}{before}▏{after}{}",
        if start > 0 { "…" } else { "" },
        if end < chars.len() { "…" } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::{candidate_preedit, pending_input_line};
    use crate::candidates::Preedit;
    use qingjian_platform::PreeditMode;

    #[test]
    fn long_pending_keys_show_the_editing_end_and_cursor_instead_of_truncated_prefix() {
        let pending = format!("{}xyz", "n".repeat(80));
        let line = pending_input_line(&pending, pending.len());
        assert!(line.starts_with("待选拼音：…"));
        assert!(line.ends_with("xyz▏"));
        assert!(line.chars().count() < 48);
        let shorter = &pending[..pending.len() - 1];
        assert!(pending_input_line(shorter, shorter.len()).ends_with("xy▏"));
        assert_eq!(pending_input_line("nihao", 2), "待选拼音：ni▏hao");
        assert_eq!(pending_input_line("你好", 3), "待选拼音：你▏好");
    }

    #[test]
    fn draft_shows_pending_keys_for_both_and_inline_without_changing_normal_preference() {
        let pending = Preedit::plain("nihao", 5).unwrap();
        let shorter = Preedit::plain("niha", 4).unwrap();
        for mode in [PreeditMode::Both, PreeditMode::Inline, PreeditMode::Window] {
            assert_eq!(
                candidate_preedit(mode, true, Some(&pending))
                    .unwrap()
                    .text(),
                "nihao"
            );
            assert_eq!(
                candidate_preedit(mode, true, Some(&shorter))
                    .unwrap()
                    .text(),
                "niha"
            );
            assert!(candidate_preedit(mode, true, None).is_none());
        }
        assert!(candidate_preedit(PreeditMode::Inline, false, Some(&pending)).is_none());
        assert!(candidate_preedit(PreeditMode::Both, false, Some(&pending)).is_some());
    }
}
