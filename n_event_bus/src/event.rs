use crate::message::{Envelope, Message, Tagged};
use std::time::Duration;

/// The bus's periodic heartbeat, dispatched like any other message so
/// subscribers can anchor polling/diffing work on it.
pub struct Tick;

impl Message for Tick {}

/// What the bus's run loop consumes.
pub enum Event {
    Bus(Envelope),
    Tick,
}

/// Clone-cheap entry point into the bus from anywhere: UI callbacks, jobs, bridges.
#[derive(Clone)]
pub struct EventWriter {
    tx: flume::Sender<Event>,
}

impl EventWriter {
    pub fn new(tx: flume::Sender<Event>) -> Self {
        Self { tx }
    }

    /// Send errors are ignored: they only happen when the app is shutting down.
    pub fn emit<T: Message>(&self, message: T) {
        let _ = self.tx.send(Event::Bus(Envelope::new(message)));
    }

    pub fn emit_tagged<P: Send + 'static>(&self, tag: u64, payload: P) {
        self.emit(Tagged::new(tag, payload));
    }
}

/// Emits [Event::Tick] on `interval` until the receiving side is dropped.
pub fn spawn_ticker(writer: EventWriter, interval: Duration) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(interval);
        loop {
            interval.tick().await;
            if writer.tx.send_async(Event::Tick).await.is_err() {
                break;
            }
        }
    });
}
