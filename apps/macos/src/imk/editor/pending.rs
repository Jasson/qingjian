//! 待填入内容必须保持同一修订；编辑、取消或译文变化均不能复用旧确认。

use qingjian_predict::live::FocusedDraftModel;

pub(super) struct Pending {
    pub id: u64,

    pub text: String,

    pub original: bool,

    pub revision: u64,

    pub attempts: u8,

    pub manual: bool,

    pub claimed: bool,
}

impl Pending {
    pub fn claim(&mut self, model: &FocusedDraftModel) -> Option<String> {
        if self.claimed || !self.matches(model) {
            return None;
        }
        self.claimed = true;
        Some(self.text.clone())
    }

    pub fn rejected(&mut self) {
        self.claimed = false;
        self.manual = true;
    }

    pub fn matches(&self, model: &FocusedDraftModel) -> bool {
        self.revision == model.revision()
            && self.text
                == if self.original {
                    model.original_output()
                } else {
                    model.translation_output()
                }
                .unwrap_or_default()
    }
}
