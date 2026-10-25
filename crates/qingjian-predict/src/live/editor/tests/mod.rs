//! 独立草稿模型回放共用的样例词库和成功回执。

mod input;
mod receipt;

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;

use crate::live::{FocusedDraftModel, Translation};

fn engine() -> Engine {
    Engine::new(
        Dictionary::parse("你好\tni hao\t10000\n你号\tni hao\t9000\n你\tni\t8000\n好\thao\t7000\n")
            .unwrap(),
    )
}

fn success(model: &mut FocusedDraftModel, id: u64, translation: &str) {
    model.submitted(id);
    assert!(model.accept(Translation {
        id,
        source: model.source(),
        result: Ok(translation.into())
    }));
}
