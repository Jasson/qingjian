//! 翻译条的独立操作按钮，禁用按钮仍展示但不提供点击区域。

mod target;

pub use target::ActionTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    pub target: ActionTarget,

    pub enabled: bool,
}
