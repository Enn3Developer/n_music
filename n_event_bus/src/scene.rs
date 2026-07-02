use crate::subscriber::Subscriber;
use crate::ui::UiPatch;
use crate::{Ctx, Outbox};

/// Optional UI-sync capability on top of [Subscriber]; dispatch never requires it.
pub trait Scene: Subscriber {
    /// Called once when the scene is registered, before the run loop starts.
    fn on_mount(&mut self, _ctx: &Ctx, _out: &mut Outbox) {}

    /// Called after a dispatch cycle that touched this scene; returns the UI
    /// mutations reflecting its current state.
    fn sync(&mut self, _ctx: &Ctx) -> Option<UiPatch> {
        None
    }
}
