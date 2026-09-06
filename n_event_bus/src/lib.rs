pub mod app;
pub mod bus;
pub mod event;
pub mod job;
pub mod message;
pub mod outbox;
pub mod registrar;
pub mod scene;
pub mod subscriber;
pub mod ui;

pub use app::App;
pub use bus::{Bus, SubscriberId};
pub use event::{Event, EventReceiver, EventWriter};
pub use job::{Job, JobControl, JobHandle, JobToken, RunningJob};
pub use message::{Envelope, Message, Tagged};
pub use outbox::Outbox;
pub use registrar::Registrar;
pub use scene::Scene;
pub use subscriber::Subscriber;
pub use ui::{UiPatch, UiThread};

use std::any::Any;

pub struct Ctx<'a> {
    pub jobs: &'a JobControl,
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
