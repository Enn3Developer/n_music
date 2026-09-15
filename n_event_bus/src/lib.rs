//! Typed, message-driven coordination with subscriber-owned asynchronous work.
//!
//! Graceful shutdown is requested with [`EventWriter::shutdown`]. Subscribers
//! opt in by registering for [`ShutdownRequested`], then acknowledge through
//! [`Outbox::shutdown_ready`] immediately or from a later cleanup-result handler.
//! Dispatch continues while any participant is pending. [`Ctx::shutting_down`]
//! lets handlers reject new user work without rejecting replies or cleanup jobs.
//!
//! Readiness is a commitment: a subscriber must not start more required work
//! after acknowledging. The bus drains accepted messages and outbox follow-ups;
//! later ingress may be discarded once everyone is ready. It does not wait for
//! job threads, and a deadline cannot interrupt a blocking handler or destructor.

pub mod app;
pub mod bus;
pub mod event;
pub mod job;
pub mod message;
pub mod outbox;
pub mod registrar;
pub mod subscriber;

pub use app::{App, ShutdownOutcome};
pub use bus::{Bus, SubscriberId};
pub use event::{Event, EventReceiver, EventWriter, ShutdownRequested};
pub use job::{Job, JobControl, JobHandle, JobToken, RunningJob};
pub use message::{Envelope, Message, Tagged};
pub use outbox::Outbox;
pub use registrar::Registrar;
pub use subscriber::Subscriber;

use std::any::Any;

pub struct Ctx<'a> {
    pub jobs: &'a JobControl,
    /// True from delivery of `ShutdownRequested` until dispatch stops.
    pub shutting_down: bool,
}

pub trait Handle<T: Message> {
    fn handle(&mut self, msg: &T, ctx: &Ctx, out: &mut Outbox);
}

pub type Thunk = fn(&mut dyn Subscriber, &dyn Any, &Ctx, &mut Outbox);

pub fn thunk<S, T>() -> Thunk
where
    S: Subscriber + Handle<T>,
    T: Message,
{
    call::<S, T>
}

fn call<S, T>(subscriber: &mut dyn Subscriber, msg: &dyn Any, ctx: &Ctx, out: &mut Outbox)
where
    S: Subscriber + Handle<T>,
    T: Message,
{
    let Some(subscriber) = subscriber.as_any_mut().downcast_mut::<S>() else {
        return;
    };
    let Some(msg) = msg.downcast_ref::<T>() else {
        return;
    };
    subscriber.handle(msg, ctx, out);
}
