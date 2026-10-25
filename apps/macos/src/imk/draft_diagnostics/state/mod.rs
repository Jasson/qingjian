//! 纯状态机：激活与进程预算、重入事件归属及写口计数。

use super::callback;
use super::frame::Frame;
use super::kind::WriteKind;
use super::record::Record;
use super::snapshot::Snapshot;

const PER_ACTIVATION: usize = 32;
const PER_PROCESS: usize = 256;
const CALLBACKS_PER_ACTIVATION: usize = 16;
const CALLBACKS_PER_PROCESS: usize = 64;

#[derive(Default)]
pub(super) struct State {
    owner: Option<usize>,

    session: u64,

    activation_events: usize,

    process_events: usize,

    activation_callbacks: usize,

    process_callbacks: usize,

    next_ticket: u64,

    frames: Vec<Frame>,
}

impl State {
    pub fn arm(&mut self, owner: usize, eligible: bool) {
        self.invalidate_frames();
        self.owner = eligible.then_some(owner);
        self.session = self.session.wrapping_add(1);
        self.activation_events = 0;
        self.activation_callbacks = 0;
    }

    pub fn disarm(&mut self, owner: usize) {
        if self.owner == Some(owner) {
            self.invalidate_frames();
            self.owner = None;
        }
    }

    fn invalidate_frames(&mut self) {
        for frame in &mut self.frames {
            frame.record = None;
        }
    }

    pub fn begin(
        &mut self,
        kind: &'static str,
        before: Option<Snapshot>,
        current: bool,
        client_present: bool,
        secure: bool,
    ) -> u64 {
        self.next_ticket = self.next_ticket.wrapping_add(1);
        let ticket = self.next_ticket;
        let eligible = !secure
            && self.owner.is_some()
            && before.is_none_or(|snapshot| snapshot.wechat)
            && self.activation_events < PER_ACTIVATION
            && self.process_events < PER_PROCESS;
        let record = eligible.then(|| {
            self.activation_events += 1;
            self.process_events += 1;
            Record {
                id: self.process_events as u64,
                session: self.session,
                kind,
                before,
                current_before: current,
                client_present,
                prepared: "not_reached",
                panicked: false,
                writes: Default::default(),
            }
        });
        self.frames.push(Frame { ticket, record });
        ticket
    }

    pub fn callback(
        &mut self,
        name: &'static str,
        before: Option<Snapshot>,
        secure: bool,
    ) -> Option<callback::Record> {
        if secure
            || self.owner.is_none()
            || before.is_some_and(|value| !value.wechat)
            || self.activation_callbacks >= CALLBACKS_PER_ACTIVATION
            || self.process_callbacks >= CALLBACKS_PER_PROCESS
        {
            return None;
        }
        self.activation_callbacks += 1;
        self.process_callbacks += 1;
        let parent = self.frames.last().and_then(|frame| frame.record.as_ref());
        Some(callback::Record {
            id: self.process_callbacks as u64,
            session: self.session,
            name,
            parent_event_id: parent.map(|record| record.id),
            parent_session: parent.map(|record| record.session),
            before,
        })
    }

    pub fn callback_current(&self, session: u64) -> bool {
        self.owner.is_some() && self.session == session
    }

    pub fn record_mut(&mut self, ticket: u64) -> Option<&mut Record> {
        self.frames
            .iter_mut()
            .find(|frame| frame.ticket == ticket)?
            .record
            .as_mut()
    }

    pub fn write(&mut self, kind: WriteKind, outcome: &'static str) {
        let Some(record) = self
            .frames
            .last_mut()
            .and_then(|frame| frame.record.as_mut())
        else {
            return;
        };
        let count = &mut record.writes[kind.index()];
        match outcome {
            "attempt" => count.attempted = count.attempted.saturating_add(1),
            "blocked" => count.blocked = count.blocked.saturating_add(1),
            "sent" => count.sent = count.sent.saturating_add(1),
            _ => {}
        }
    }

    pub fn finish(&mut self, ticket: u64) -> Option<Record> {
        let index = self
            .frames
            .iter()
            .position(|frame| frame.ticket == ticket)?;
        self.frames.remove(index).record
    }
}

#[cfg(test)]
mod tests;
