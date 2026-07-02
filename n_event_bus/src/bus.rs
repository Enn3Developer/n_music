use crate::message::Envelope;
use crate::Thunk;
use std::any::TypeId;
use std::collections::{HashMap, VecDeque};

/// Registration identity for any subscriber, scene or not.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct SubscriberId(pub(crate) u64);

/// `TypeId`-indexed subscriptions plus the pending message queue.
#[derive(Default)]
pub struct Bus {
    index: HashMap<TypeId, Vec<(SubscriberId, Thunk)>>,
    queue: VecDeque<Envelope>,
}

impl Bus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&mut self, tid: TypeId, id: SubscriberId, thunk: Thunk) {
        self.index.entry(tid).or_default().push((id, thunk));
    }

    pub fn enqueue(&mut self, envelope: Envelope) {
        self.queue.push_back(envelope);
    }

    pub fn pop(&mut self) -> Option<Envelope> {
        self.queue.pop_front()
    }

    pub fn subscribers_for(&self, tid: TypeId) -> Vec<(SubscriberId, Thunk)> {
        self.index.get(&tid).cloned().unwrap_or_default()
    }
}
