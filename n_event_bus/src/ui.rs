/// One retained-mode UI mutation, built by [crate::Scene::sync] off the UI
/// thread; [UiThread::apply] guarantees it runs on the UI thread.
pub type UiPatch = Box<dyn FnOnce() + Send>;

/// The one seam to the UI framework: applies a batch of patches on the UI
/// thread in a single hop (e.g. `slint::invoke_from_event_loop`).
pub trait UiThread: Send + Sync + 'static {
    fn apply(&self, patches: Vec<UiPatch>);
}
