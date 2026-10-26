//! 草稿退格的内部编辑顺序与恢复；只修改输入法状态，不调用文本客户端。

use qingjian_core::Engine;
use qingjian_predict::live::TerminalDraft;

/// 返回编辑目标是否为拼音、是否发生删除；拼音目标需要重新查询候选。
pub(super) fn backspace(engine: &mut Engine, draft: &mut TerminalDraft) -> (bool, bool) {
    let composing = !engine.composition().is_empty();
    let changed = if composing {
        engine.backspace()
    } else {
        let before = draft.source();
        draft.backspace();
        before != draft.source()
    };
    (composing, changed)
}

pub(super) fn backspace_batch(engine: &mut Engine, draft: &mut TerminalDraft, count: u16) -> bool {
    let mut changed = false;
    for _ in 0..count {
        changed |= backspace(engine, draft).1;
        if engine.composition().is_empty() {
            draft.preview("");
        }
    }
    changed
}

pub(super) fn resume(paused: &mut bool, source: &mut String) -> bool {
    if !*paused {
        return false;
    }
    *paused = false;
    source.clear();
    true
}

#[cfg(test)]
mod tests {
    use super::{backspace, backspace_batch, resume};
    use qingjian_core::Engine;
    use qingjian_dictionary::Dictionary;
    use qingjian_predict::live::TerminalDraft;

    #[test]
    fn pinyin_is_edited_before_confirmed_draft_and_repeated_backspace_reaches_the_draft() {
        let mut engine = Engine::new(Dictionary::parse("你\tni\t1\n").unwrap());
        let mut draft = TerminalDraft::default();
        draft.commit("已经确认");
        engine.push('n');
        engine.push('i');
        draft.preview("你");
        assert_eq!(backspace(&mut engine, &mut draft), (true, true));
        assert_eq!(engine.composition().text(), "n");
        assert_eq!(draft.committed(), "已经确认");
        assert_eq!(backspace(&mut engine, &mut draft), (true, true));
        assert!(engine.composition().is_empty());
        // refresh 会把候选预览清空，不把它误当已确认原文删除。
        draft.preview("");
        assert_eq!(draft.source(), "已经确认");
        assert_eq!(backspace(&mut engine, &mut draft), (false, true));
        assert_eq!(draft.source(), "已经确");
        for _ in 0..4 {
            backspace(&mut engine, &mut draft);
        }
        assert!(draft.source().is_empty());
    }

    #[test]
    fn backspace_at_pinyin_start_reports_no_change() {
        let mut engine = Engine::new(Dictionary::parse("你\tni\t1\n").unwrap());
        let mut draft = TerminalDraft::default();
        draft.commit("已确认");
        engine.push('n');
        engine.move_cursor_home();
        assert_eq!(backspace(&mut engine, &mut draft), (true, false));
        assert_eq!(engine.composition().text(), "n");
        assert_eq!(draft.committed(), "已确认");
    }

    #[test]
    fn paused_draft_resumes_without_importing_cached_source_and_can_edit_immediately() {
        let mut engine = Engine::new(Dictionary::parse("").unwrap());
        let mut draft = TerminalDraft::default();
        draft.commit("当前草稿");
        let mut paused = true;
        let mut source = "上次缓存的不同原文".to_owned();
        assert!(resume(&mut paused, &mut source));
        assert!(!paused);
        assert!(source.is_empty());
        assert_eq!(backspace(&mut engine, &mut draft), (false, true));
        assert_eq!(draft.source(), "当前草");
        assert!(!resume(&mut paused, &mut source));
    }

    #[test]
    fn batch_crosses_pinyin_boundary_without_deleting_preview_as_confirmed_text() {
        let mut engine = Engine::new(Dictionary::parse("你\tni\t1\n").unwrap());
        let mut draft = TerminalDraft::default();
        draft.commit("已经确认");
        engine.push('n');
        engine.push('i');
        draft.preview("你");
        backspace_batch(&mut engine, &mut draft, 4);
        assert!(engine.composition().is_empty());
        assert_eq!(draft.committed(), "已经");
        assert_eq!(draft.source(), "已经");
        backspace_batch(&mut engine, &mut draft, 100);
        assert!(draft.source().is_empty());
        assert!(engine.composition().is_empty());
    }

    #[test]
    fn original_output_uses_actual_case_and_shuangpin_keys_without_candidate_preview() {
        let mut engine = Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap());
        let mut draft = TerminalDraft::default();
        draft.commit("今天");
        draft.preview("你好");
        engine.set_shift_letter_compose(true);
        engine.set_input("NiHao");
        assert_eq!(engine.composition().text(), "nihao");
        assert_eq!(
            draft
                .original_output_with_pending(&engine.raw_preedit().text)
                .as_deref(),
            Some("今天NiHao")
        );
        engine.clear();
        engine.set_shuangpin(Some(qingjian_core::ShuangpinScheme::Xiaohe));
        engine.set_input("nihc");
        assert_eq!(engine.query().unwrap().marked_text(), "ni'hao");
        assert_eq!(
            draft
                .original_output_with_pending(&engine.raw_preedit().text)
                .as_deref(),
            Some("今天nihc")
        );
        assert_eq!(draft.source(), "今天你好");
    }
}
