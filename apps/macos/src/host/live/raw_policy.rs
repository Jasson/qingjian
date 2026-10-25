//! 待选原始键重新解释所需的配置指纹；变化后旧键序不能自动套新输入方案。

use qingjian_platform::{GeneralConfig, Scheme};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct RawPolicy {
    scheme: Scheme,

    wubi: bool,

    shift_compose: bool,

    english: bool,
}

impl RawPolicy {
    pub fn for_config(config: &GeneralConfig) -> Self {
        Self {
            scheme: config.scheme(),
            wubi: config.wubi(),
            shift_compose: config.shift_letter.compose(),
            english: config.english_mode,
        }
    }
}
