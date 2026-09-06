pub type UiPatch = Box<dyn FnOnce() + Send>;

pub trait UiThread: Send + Sync + 'static {
    fn apply(&self, patches: Vec<UiPatch>);
}
