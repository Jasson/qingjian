//! 草稿的明确写入目的地；不能把 controller 或代理地址单独当成输入框身份证明。

mod field;

use crate::host;
use crate::imk::{QingjianInputController, TextClient, controller, frontmost, secure_input};
use field::Field;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use qingjian_predict::live::context::ContextSnapshot;

pub(super) struct Target {
    pub controller: Retained<QingjianInputController>,

    pub client: Retained<AnyObject>,

    pub application: String,

    pub pid: i32,

    metadata: ContextSnapshot,

    field: Option<Field>,

    capture: host::ObservationToken,
}

impl Target {
    pub fn capture(
        controller: Retained<QingjianInputController>,
        client: TextClient<'_>,
    ) -> Option<Self> {
        if secure_input::enabled() {
            return None;
        }
        let observation = host::with(|h| h.live_observation_token())?;
        let (application, pid) = frontmost::identity()?;
        let owner = &*controller as *const QingjianInputController as usize;
        let current = || {
            !secure_input::enabled()
                && controller::active_controller_id() == Some(owner)
                && host::with(|h| observation.matches(&h.live_observation_token())).unwrap_or(false)
        };
        let metadata = client.live_selection_metadata(current)?;
        let valid = current()
            && pid > 0
            && metadata.application.as_deref() == Some(application.as_str())
            && valid_metadata(&metadata);
        if !valid {
            return None;
        }
        let field = Field::capture(pid, metadata.selected?);
        if !current() || frontmost::identity() != Some((application.clone(), pid)) {
            return None;
        }
        Some(Self {
            controller,
            client: client.retained_object(),
            application,
            pid,
            metadata,
            field,
            capture: observation,
        })
    }

    pub fn is_current(&self) -> bool {
        !secure_input::enabled()
            && frontmost::identity() == Some((self.application.clone(), self.pid))
            && controller::active_controller_id()
                == Some(&*self.controller as *const QingjianInputController as usize)
    }

    pub fn verify(&self, manual_confirmation: bool, guard: &impl Fn() -> bool) -> bool {
        let Some(observation) = (if manual_confirmation {
            Some(self.capture.clone())
        } else {
            host::with(|h| h.live_observation_token())
        }) else {
            return false;
        };
        let stable = || {
            guard()
                && self.is_current()
                && host::with(|h| observation.matches(&h.live_observation_token())).unwrap_or(false)
        };
        if !stable()
            || !self.field.as_ref().map_or(manual_confirmation, |field| {
                self.metadata
                    .selected
                    .is_some_and(|range| field.matches(self.pid, range))
            })
        {
            return false;
        }
        let actual: Option<Retained<AnyObject>> = unsafe { msg_send![&*self.controller, client] };
        if !actual
            .as_deref()
            .is_some_and(|client| std::ptr::eq(client, &*self.client))
        {
            return false;
        }
        let client = TextClient::new(&self.client);
        let Some(metadata) = client.live_selection_metadata(stable) else {
            return false;
        };
        let field_stable = self.field.as_ref().map_or(manual_confirmation, |field| {
            self.metadata
                .selected
                .is_some_and(|range| field.matches(self.pid, range))
        });
        stable()
            && field_stable
            && valid_metadata(&metadata)
            && metadata.application == self.metadata.application
            && metadata.client_proxy == self.metadata.client_proxy
            && metadata.selected == self.metadata.selected
            && metadata.active_marked().is_none()
            && metadata.document_length == self.metadata.document_length
    }

    pub fn insert(&self, text: &str, manual_confirmation: bool, guard: impl Fn() -> bool) -> bool {
        if !self.verify(manual_confirmation, &guard)
            || !guard()
            || !self.is_current()
            || !host::with(|h| h.settings.config().live_translation.enabled).unwrap_or(false)
            || !guard()
        {
            return false;
        }
        TextClient::new(&self.client).insert_confirmed_draft(text)
    }
}

fn valid_metadata(metadata: &ContextSnapshot) -> bool {
    metadata.document_length <= 1_048_576
        && metadata
            .selected
            .is_some_and(|range| range.length == 0 && range.within(metadata.document_length))
        && metadata.active_marked().is_none()
}
