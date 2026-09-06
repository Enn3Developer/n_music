use crate::registrar::Registrar;
use std::any::Any;

pub trait Subscriber: Send + 'static {
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn register(reg: &mut Registrar<Self>)
    where
        Self: Sized;
}
