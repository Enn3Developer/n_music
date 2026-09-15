use crate::message::{Envelope, Message, Tagged};

/// Begins graceful shutdown. Subscribers registered for this message must call
/// `Outbox::shutdown_ready()` when their required cleanup has finished.
///
/// The bus delivers this message once, even if shutdown is requested repeatedly.
/// Subscribers without this subscription do not delay shutdown.
pub struct ShutdownRequested;

impl Message for ShutdownRequested {}

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

    /// Requests subscriber-coordinated shutdown without waiting for completion.
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
