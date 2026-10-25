//! 事件身份初始化结果；无法核实的当前会话按下事件不能落入宿主提交动作。

use crate::imk::TextClient;

pub(super) enum PreparedClient<'a> {
    Ready(TextClient<'a>),

    Consume,

    Pass,
}

impl PreparedClient<'_> {
    pub fn uncertain(key_down: bool, current: bool) -> Self {
        if key_down && current {
            Self::Consume
        } else {
            Self::Pass
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PreparedClient;

    #[test]
    fn busy_or_reentered_preparation_keeps_current_enter_from_reaching_the_host() {
        assert!(matches!(
            PreparedClient::uncertain(true, true),
            PreparedClient::Consume
        ));
        assert!(matches!(
            PreparedClient::uncertain(true, false),
            PreparedClient::Pass
        ));
        assert!(matches!(
            PreparedClient::uncertain(false, true),
            PreparedClient::Pass
        ));
    }
}
