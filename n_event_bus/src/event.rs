use crate::message::{Envelope, Message, Tagged};

pub enum Event {
    Bus(Envelope),
    Shutdown,
}

#[derive(Clone)]
pub struct EventWriter {
    tx: flume::Sender<Event>,
}

impl EventWriter {
    pub fn channel() -> (Self, EventReceiver) {
        let (tx, rx) = flume::unbounded();
        (Self::new(tx), rx)
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(Event::Shutdown);
    }

    pub fn new(tx: flume::Sender<Event>) -> Self {
        Self { tx }
    }

    pub fn emit<T: Message>(&self, message: T) {
        let _ = self.tx.send(Event::Bus(Envelope::new(message)));
    }

    pub fn emit_tagged<P: Send + 'static>(&self, tag: u64, payload: P) {
        self.emit(Tagged::new(tag, payload));
    }
}

pub type EventReceiver = flume::Receiver<Event>;
