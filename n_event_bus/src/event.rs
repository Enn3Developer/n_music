use crate::message::{Envelope, Message, Tagged};
use std::time::Duration;

pub struct Tick;

impl Message for Tick {}

pub enum Event {
    Bus(Envelope),
    Tick,
}

#[derive(Clone)]
pub struct EventWriter {
    tx: flume::Sender<Event>,
}

impl EventWriter {
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
