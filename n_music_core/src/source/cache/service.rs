//! The stream cache on the bus.

use super::StreamCache;
use crate::library::catalog::Library;
use crate::messages::{SetStreamCache, TrackPlayed, TracksEnumerated};
use crate::settings::{LibrarySettings, Options};
use n_event_bus::{Ctx, Handle, Outbox, Registrar, Subscriber};
use std::any::Any;
use std::sync::Arc;

/// Applies the stream cache's settings, and deletes the copies the library has no track for.
pub(crate) struct StreamCacheService {
    cache: Arc<StreamCache>,
    library: Library,
    libraries: Options<LibrarySettings>,
}

impl StreamCacheService {
    pub(crate) fn new(
        cache: Arc<StreamCache>,
        library: Library,
        libraries: Options<LibrarySettings>,
    ) -> Self {
        Self {
            cache,
            library,
            libraries,
        }
    }
}

impl Subscriber for StreamCacheService {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<SetStreamCache>();
        reg.on::<TrackPlayed>();
        reg.on::<TracksEnumerated>();
    }
}

impl Handle<SetStreamCache> for StreamCacheService {
    fn handle(&mut self, msg: &SetStreamCache, _ctx: &Ctx, _out: &mut Outbox) {
        self.cache.configure(msg.enabled, msg.limit);
    }
}

/// The library service, registered before, counted the play already.
impl Handle<TrackPlayed> for StreamCacheService {
    fn handle(&mut self, msg: &TrackPlayed, _ctx: &Ctx, _out: &mut Outbox) {
        self.cache.played(&msg.locator);
    }
}

impl Handle<TracksEnumerated> for StreamCacheService {
    fn handle(&mut self, _msg: &TracksEnumerated, _ctx: &Ctx, _out: &mut Outbox) {
        let libraries = self.libraries.get().libraries.clone();
        // Listed in this launch: what an earlier one listed may lack tracks that have a copy.
        let whole = {
            let catalog = self.library.read();
            libraries.iter().all(|library| {
                catalog
                    .listed(library)
                    .is_some_and(|listed| listed.reachable && !listed.restored)
            })
        };
        if whole {
            self.cache.prune(&libraries);
        }
    }
}
