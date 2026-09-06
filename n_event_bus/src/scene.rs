use crate::subscriber::Subscriber;
use crate::ui::UiPatch;
use crate::{Ctx, Outbox};

pub trait Scene: Subscriber {
    fn on_mount(&mut self, _ctx: &Ctx, _out: &mut Outbox) {}

    fn sync(&mut self, _ctx: &Ctx) -> Option<UiPatch> {
        None
    }
}
