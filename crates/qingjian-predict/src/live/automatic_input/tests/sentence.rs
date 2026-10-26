//! 真实断句器与启动状态联合回放，后一句不能继承前一句的字母／汉字资格。

use crate::live::{AutomaticSource, SentenceTracker};

fn observe(automatic: &mut AutomaticSource, tracker: &SentenceTracker, raw: &str) -> bool {
    let (source, start) = tracker.source_with_start();
    automatic.observe_sentence(&source, raw, start)
}

#[test]
fn a_new_numeric_sentence_starts_only_when_it_gains_a_letter_and_keeps_its_history() {
    let mut tracker = SentenceTracker::new();
    let mut automatic = AutomaticSource::default();
    tracker.commit("你好。");
    assert!(observe(&mut automatic, &tracker, ""));
    tracker.commit("123");
    assert_eq!(tracker.source(), "123");
    assert!(!observe(&mut automatic, &tracker, ""));
    assert!(!automatic.visible(&tracker.source(), "", false));
    tracker.commit("a");
    assert!(observe(&mut automatic, &tracker, ""));
    tracker.backspace();
    assert_eq!(tracker.source(), "123");
    assert!(observe(&mut automatic, &tracker, ""));
    assert!(automatic.visible(&tracker.source(), "", false));
}

#[test]
fn a_new_emoji_sentence_cannot_inherit_the_preceding_chinese_sentence() {
    let mut tracker = SentenceTracker::new();
    let mut automatic = AutomaticSource::default();
    tracker.commit("你好。");
    assert!(observe(&mut automatic, &tracker, ""));
    tracker.commit("🙂");
    assert_eq!(tracker.source(), "🙂");
    assert!(!observe(&mut automatic, &tracker, ""));
    assert!(!automatic.visible(&tracker.source(), "", false));
}

#[test]
fn committing_a_preview_cannot_carry_its_qualification_across_the_sentence_boundary() {
    let mut tracker = SentenceTracker::new();
    let mut automatic = AutomaticSource::default();
    tracker.preview("你好。");
    assert!(observe(&mut automatic, &tracker, "nihao"));
    tracker.commit("你好。");
    assert!(observe(&mut automatic, &tracker, ""));
    tracker.preview("123");
    assert!(!observe(&mut automatic, &tracker, ""));
    tracker.commit("123");
    assert!(!observe(&mut automatic, &tracker, ""));
    assert!(!automatic.visible(&tracker.source(), "", false));
}
