use crate::message::Message;
use crate::subscriber::Subscriber;
use crate::{thunk, Handle, Thunk};
use std::any::TypeId;
use std::marker::PhantomData;

/// Collects the `(TypeId, Thunk)` pairs a subscriber type declares in
/// [Subscriber::register]; [crate::App::register_subscriber] feeds them into the bus index.
pub struct Registrar<S: Subscriber> {
    pairs: Vec<(TypeId, Thunk)>,
    _marker: PhantomData<fn(S)>,
}

impl<S: Subscriber> Registrar<S> {
    pub fn new() -> Self {
        Self {
            pairs: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn on<T: Message>(&mut self)
    where
        S: Handle<T>,
    {
        self.pairs.push((TypeId::of::<T>(), thunk::<S, T>()));
    }

    pub fn into_pairs(self) -> Vec<(TypeId, Thunk)> {
        self.pairs
    }
}

impl<S: Subscriber> Default for Registrar<S> {
    fn default() -> Self {
        Self::new()
    }
}
