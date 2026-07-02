use std::any::{Any, TypeId};

/// Marker trait for anything that can travel on the bus.
pub trait Message: Any + Send {}

/// A message carrying a job tag so subscribers can drop results from superseded jobs.
pub struct Tagged<P> {
    tag: u64,
    payload: P,
}

impl<P: Send + 'static> Message for Tagged<P> {}

impl<P> Tagged<P> {
    pub fn new(tag: u64, payload: P) -> Self {
        Self { tag, payload }
    }

    /// Returns the payload only if `tag` matches the one this message was emitted with.
    pub fn open(&self, tag: u64) -> Option<&P> {
        (self.tag == tag).then_some(&self.payload)
    }
}

/// A type-erased message together with the [TypeId] the bus dispatches on.
pub struct Envelope {
    tid: TypeId,
    payload: Box<dyn Any + Send>,
}

impl Envelope {
    pub fn new<T: Message>(message: T) -> Self {
        Self {
            tid: TypeId::of::<T>(),
            payload: Box::new(message),
        }
    }

    pub fn tid(&self) -> TypeId {
        self.tid
    }

    pub fn payload(&self) -> &(dyn Any + Send) {
        self.payload.as_ref()
    }
}
