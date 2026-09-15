use crate::message::{Envelope, Message};

#[derive(Default)]
pub struct Outbox {
    envelopes: Vec<Envelope>,
    pub(crate) shutdown_ready: bool,
}

impl Outbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emit<T: Message>(&mut self, message: T) {
        self.envelopes.push(Envelope::new(message));
    }

    /// Acknowledges shutdown for the subscriber handling the current message.
    ///
    /// Call only after required replies have arrived and cleanup has settled.
    /// Final notifications belong in this outbox: the bus dispatches them before
    /// stopping. Duplicate acknowledgements and those before shutdown are ignored.
    pub fn shutdown_ready(&mut self) {
        self.shutdown_ready = true;
    }

    pub fn into_envelopes(self) -> Vec<Envelope> {
        self.envelopes
    }
}
