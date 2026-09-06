use crate::message::{Envelope, Message};

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
