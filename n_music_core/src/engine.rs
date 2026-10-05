//! The core services, wired onto a frontend's bus.

use crate::library::catalog::Library;
use crate::library::service::LibraryService;
use crate::library::LibraryPaths;
use crate::messages::{LoopStatusChanged, ScanRequested, ShuffleChanged, VolumeChanged};
use crate::queue::QueuePlayer;
use crate::settings::{LibrarySettings, Options, PlaybackSettings, SettingsStorage};
use crate::source::Providers;
use n_event_bus::{App, EventWriter};
use std::path::Path;
use std::sync::Arc;

/// What frontends share with the core services.
pub struct Engine {
    library: Library,
}

impl Engine {
    /// Registers the library and the player on `app`, then queues the startup messages: the
    /// playback state (volume, loop, shuffle) and a scan, which also reports the library
    /// folders and the playlists. They are handled once the loop runs, so subscribers
    /// registered after this still get them.
    ///
    /// `data_dir` keeps what the user creates (the library database); `cache_dir` what can be
    /// rebuilt (covers).
    pub fn start(
        app: &mut App,
        writer: &EventWriter,
        storage: Arc<dyn SettingsStorage>,
        providers: Arc<Providers>,
        data_dir: &Path,
        cache_dir: &Path,
    ) -> Self {
        let paths = LibraryPaths::new(data_dir, cache_dir);
        let libraries = Options::<LibrarySettings>::load(storage.clone());
        let playback = Options::<PlaybackSettings>::load(storage);
        let service = LibraryService::new(providers.clone(), libraries, paths);
        let library = service.library();
        let player = QueuePlayer::new(providers, playback, library.clone());
        writer.emit(VolumeChanged(player.volume()));
        writer.emit(LoopStatusChanged(player.loop_status()));
        writer.emit(ShuffleChanged(player.shuffle()));
        writer.emit(ScanRequested {
            library: None,
            check_cache: true,
        });
        app.register_subscriber(service);
        app.register_subscriber(player);
        Self { library }
    }

    /// The library the services keep up to date.
    pub fn library(&self) -> Library {
        self.library.clone()
    }
}
