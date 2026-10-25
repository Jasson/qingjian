//! 草稿按应用隔离，最近优先取出，容量固定且不丢失纯空白。

use super::DraftArchive;

#[test]
fn applications_are_isolated_and_the_latest_draft_is_removed_once() {
    let mut archive = DraftArchive::new();
    assert!(archive.stash("browser", "你好", "shi"));
    assert!(archive.stash("editor", "other", ""));
    assert!(archive.stash("browser", "你好世界", "a"));
    assert!(archive.take_for("unknown").is_none());
    let saved = archive.take_for("browser").unwrap();
    assert_eq!(saved.application, "browser");
    assert_eq!(saved.committed, "你好世界");
    assert_eq!(saved.raw_keys, "a");
    assert_eq!(archive.take_for("browser").unwrap().committed, "你好");
    assert!(!archive.has_for("browser"));
    assert!(archive.take_for("browser").is_none());
    assert_eq!(archive.take_for("editor").unwrap().committed, "other");
}

#[test]
fn only_the_oldest_of_seventeen_entries_is_evicted() {
    let mut archive = DraftArchive::new();
    for i in 0..17 {
        assert!(archive.stash(&format!("app{i}"), &i.to_string(), ""));
    }
    assert!(!archive.has_for("app0"));
    for i in 1..17 {
        assert_eq!(
            archive.take_for(&format!("app{i}")).unwrap().committed,
            i.to_string()
        );
    }
}

#[test]
fn consecutive_duplicates_are_ignored_but_whitespace_and_raw_keys_are_content() {
    let mut archive = DraftArchive::new();
    assert!(!archive.stash("app", "", ""));
    assert!(archive.stash("app", " \t\n", ""));
    assert!(!archive.stash("app", " \t\n", ""));
    assert!(archive.stash("app", " \t\n", "x"));
    assert!(archive.stash("other", " \t\n", "x"));
    assert!(archive.stash("app", "", "ni"));
    assert_eq!(archive.take_for("app").unwrap().raw_keys, "ni");
    assert_eq!(archive.take_for("app").unwrap().raw_keys, "x");
    assert_eq!(archive.take_for("app").unwrap().committed, " \t\n");
    archive.clear();
    assert!(!archive.has_for("other"));
    assert!(archive.take_for("other").is_none());
}
