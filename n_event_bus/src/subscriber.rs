use crate::registrar::Registrar;
use std::any::Any;

/// A thing the bus can dispatch messages to.
///
/// `register` is where a type declares the message types it handles via
/// [Registrar::on], one call per `impl Handle<T>` it provides (or by delegating
/// to a job's `job_emits!`-generated `subscribe`).
pub trait Subscriber: Send + 'static {
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn register(reg: &mut Registrar<Self>)
    where
        Self: Sized;
}
