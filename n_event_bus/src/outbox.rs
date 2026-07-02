use crate::message::{Envelope, Message};

/// Collects follow-up messages emitted by a handler; the bus enqueues them
/// after the handler returns instead of the handler poking other subscribers directly.
#[derive(Default)]
pub struct Outbox {
    envelopes: Vec<Envelope>,
}

impl Outbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emit<T: Message>(&mut self, message: T) {
        self.envelopes.push(Envelope::new(message));
    }

    pub fn into_envelopes(self) -> Vec<Envelope> {
        self.envelopes
    }
}
