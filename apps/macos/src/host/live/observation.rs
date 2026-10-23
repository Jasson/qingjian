//! 客户端 IPC 前后的受控输入状态凭据；仅短期比较，禁止写入日志。

use crate::host::Host;

#[derive(Clone, PartialEq, Eq)]
pub struct ObservationToken {
    epoch: u64,

    source: String,

    request_id: Option<u64>,

    composition: String,

    raw_composition: String,

    cursor: usize,

    committed: String,

    paused: bool,

    draft_revision: u64,
}

impl ObservationToken {
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn matches(&self, current: &Self) -> bool {
        self == current
    }
}

impl Host {
    pub fn live_observation_token(&self) -> ObservationToken {
        ObservationToken {
            epoch: self.live.epoch,
            source: self.live.source.clone(),
            request_id: self.live.request_id,
            composition: self.engine.composition().text().to_owned(),
            raw_composition: self.engine.raw_preedit().text,
            cursor: self.engine.composition().cursor(),
            committed: self.live.tracker.committed().to_owned(),
            paused: self.live.paused,
            draft_revision: self.live.draft.revision(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ObservationToken;

    fn token() -> ObservationToken {
        ObservationToken {
            epoch: 1,
            source: "你说的都对".to_owned(),
            request_id: Some(2),
            composition: "ni".to_owned(),
            raw_composition: "ni".to_owned(),
            cursor: 2,
            committed: "你说的都对".to_owned(),
            paused: false,
            draft_revision: 0,
        }
    }

    #[test]
    fn same_epoch_composition_commit_invalidates_old_snapshot_without_focus_change() {
        let before = token();
        let mut current = before.clone();
        current.composition.clear();
        current.cursor = 0;
        current.committed.push('你');
        assert_eq!(before.epoch(), current.epoch());
        assert!(!before.matches(&current));
        assert_eq!(current.committed, "你说的都对你");
        assert!(current.composition.is_empty());
    }

    #[test]
    fn whitespace_commit_changes_token_even_if_trimmed_source_and_request_are_unchanged() {
        let before = token();
        let mut current = before.clone();
        current.committed.push(' ');
        assert_eq!(before.source, current.source);
        assert_eq!(before.request_id, current.request_id);
        assert!(!before.matches(&current));
    }

    #[test]
    fn request_source_cursor_and_focus_each_invalidate_the_observation() {
        let before = token();
        assert!(before.matches(&before.clone()));
        for change in 0..7 {
            let mut current = before.clone();
            match change {
                0 => current.request_id = Some(3),
                1 => current.source.push('你'),
                2 => current.cursor = 1,
                3 => current.epoch = 2,
                4 => current.paused = true,
                5 => current.draft_revision += 1,
                _ => current.raw_composition = "Ni".to_owned(),
            }
            assert!(!before.matches(&current));
        }
    }
}
