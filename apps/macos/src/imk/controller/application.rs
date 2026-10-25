//! 事件开始时先核实应用身份，再创建本次写入门禁，避免继承旧应用的草稿缓存。

use objc2::runtime::AnyObject;

use super::QingjianInputController;
use super::prepared_client::PreparedClient;
use crate::host;
use crate::imk::TextClient;

impl QingjianInputController {
    pub(super) fn client_for_event<'a>(
        &self,
        object: &'a AnyObject,
        key_down: bool,
    ) -> PreparedClient<'a> {
        let Some(observation) = host::with(|h| h.live_observation_token()) else {
            return PreparedClient::uncertain(key_down, self.is_current_controller());
        };
        let application = TextClient::new(object).bundle_identifier();
        if !self.is_current_controller() {
            return PreparedClient::Pass;
        }
        let accepted = host::with(|h| {
            if !observation.matches(&h.live_observation_token()) {
                return false;
            }
            h.accept_live_application(application.as_deref());
            true
        });
        if accepted == Some(true) && self.is_current_controller() {
            PreparedClient::Ready(TextClient::new(object))
        } else {
            PreparedClient::uncertain(key_down, self.is_current_controller())
        }
    }
}
