//! 真正的 IMK 写入替身：草稿模式阻断全部普通写口，只有确认单行输出发送一次。

use super::selection::SelectionClient;
use crate::imk::TextClient;
use objc2::DeclaredClass;
use objc2_foundation::NSRange;
use qingjian_predict::live::TerminalDraft;

#[test]
fn staged_selections_english_punctuation_mode_switch_and_marked_never_write() {
    let mock = SelectionClient::new();
    let client = TextClient {
        object: &mock,
        stage_output: true,
    };
    let mut draft = TerminalDraft::default();
    for selected in ["你好", "。下一句", " English", "，", "多字符输入"] {
        draft.preview(selected);
        client.set_marked_text("pinyin", 6);
        client.commit_with(selected, |text| {
            assert!(draft.commit(text));
        });
        // 普通 refresh 的空 marked 也不能发送给 Terminal。
        client.set_marked_text("", 0);
    }
    client.commit_pending_raw("mode-switch-raw");
    client.replace_range("selection-review", NSRange::new(0, 1));
    assert_eq!(draft.source(), "你好。下一句 English，多字符输入");
    assert!(mock.ivars().borrow().is_empty());
    // 即使上层 reset/focus/panic 后调用原文写口，捕获的 stage 标志仍然拦截。
    draft.reset();
    client.insert_text("panic-raw");
    client.commit_pending_raw("implicit-commit");
    assert!(mock.ivars().borrow().is_empty());
}

#[test]
fn confirmed_output_is_one_plain_insert_without_line_breaks_or_followup_marked() {
    let mock = SelectionClient::new();
    let client = TextClient {
        object: &mock,
        stage_output: true,
    };
    for rejected in ["a\nb", "a\rb", "\u{1b}[31m", "\t", "", "  "] {
        assert!(!client.insert_confirmed_draft(rejected));
    }
    assert!(mock.ivars().borrow().is_empty());
    assert!(client.insert_confirmed_draft("Hello. Next sentence."));
    assert_eq!(&*mock.ivars().borrow(), &["insert:Hello. Next sentence."]);
}
