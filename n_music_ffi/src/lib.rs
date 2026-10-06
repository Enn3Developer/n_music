//! The core for Kotlin: UniFFI generates the Kotlin side of everything exported here.
//!
//! Kotlin reaches the bus through [`Core`]: [`Core::send`] emits the message a [`Command`]
//! stands for, [`Core::next_event`] hands over what the core announced as [`CoreEvent`]s, and
//! the queries read the library directly, off the bus, like the desktop's worker.

#[cfg(target_os = "android")]
mod android;
mod bridge;
mod commands;
mod rows;
mod types;

pub use bridge::CoreEvent;
pub use commands::Command;
pub use rows::{QueueRow, TrackRow};

use bridge::KotlinBridge;
use n_event_bus::{App, EventWriter, JobControl, ShutdownOutcome};
use n_music_core::engine::Engine;
use n_music_core::library::catalog::Library;
use n_music_core::library::query::Query;
use n_music_core::settings::JsonFileStorage;
use n_music_core::source::Providers;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

uniffi::setup_scaffolding!();

static CORE: OnceLock<Arc<Core>> = OnceLock::new();

/// The core services on their bus, for the whole life of the process.
#[derive(uniffi::Object)]
pub struct Core {
    writer: EventWriter,
    events: flume::Receiver<CoreEvent>,
    library: Library,
    library_changed: Arc<AtomicBool>,
}

#[uniffi::export]
impl Core {
    /// Starts the core, once per process: later calls return the core already running.
    ///
    /// `data_dir` keeps what the user creates (settings, the library database, logs);
    /// `cache_dir` what can be rebuilt (covers, copies of streamed tracks). On Android,
    /// `NativeLibrary.init` must have run first.
    #[uniffi::constructor]
    pub fn start(data_dir: String, cache_dir: String) -> Arc<Self> {
        CORE.get_or_init(|| Arc::new(Core::launch(Path::new(&data_dir), Path::new(&cache_dir))))
            .clone()
    }

    /// Sends `command` to the core. It never blocks, so the main thread can call it.
    pub fn send(&self, command: Command) {
        command.emit(&self.writer);
    }

    /// The next thing the core announced, waiting for one; `None` once the core stopped.
    /// Meant for a single reader: each event goes to one caller.
    pub async fn next_event(&self) -> Option<CoreEvent> {
        let event = self.events.recv_async().await.ok()?;
        Some(self.taken(event))
    }

    /// The tracks `query` selects, in its order: `limit` of them from `offset`.
    pub fn tracks(&self, query: Query, offset: u32, limit: u32) -> Vec<TrackRow> {
        let catalog = self.library.read();
        catalog
            .select(&query)
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|track| TrackRow::from(&**track))
            .collect()
    }

    /// How many tracks `query` selects.
    pub fn count(&self, query: Query) -> u32 {
        let count = self.library.read().select(&query).len();
        u32::try_from(count).unwrap_or(u32::MAX)
    }
}

impl Core {
    /// Notes that Kotlin took `event`.
    fn taken(&self, event: CoreEvent) -> CoreEvent {
        if matches!(event, CoreEvent::LibraryChanged) {
            // Changes from now on announce themselves again.
            self.library_changed.store(false, Ordering::Release);
        }
        event
    }

    fn launch(data_dir: &Path, cache_dir: &Path) -> Self {
        for dir in [data_dir, cache_dir] {
            if let Err(error) = std::fs::create_dir_all(dir) {
                eprintln!("Could not create {}: {error}", dir.display());
            }
        }
        match n_music_core::logging::init(data_dir) {
            // The logger lives as long as the process.
            Ok(logging) => std::mem::forget(logging),
            Err(error) => eprintln!("Could not initialize logging: {error}"),
        }
        let storage = Arc::new(JsonFileStorage::open(settings_path(data_dir)));

        let (writer, rx) = EventWriter::channel();
        let mut app = App::new(JobControl::new(writer.clone()));
        let engine = Engine::start(&mut app, &writer, storage, providers(), data_dir, cache_dir);
        let library = engine.library();
        let (events, receiver) = flume::unbounded();
        let library_changed = Arc::new(AtomicBool::new(false));
        // After the core services, so the library is up to date when Kotlin hears.
        app.register_subscriber(KotlinBridge::new(
            events,
            library.clone(),
            library_changed.clone(),
        ));
        std::thread::Builder::new()
            .name(String::from("n_event_bus loop"))
            .spawn(move || {
                let outcome = app.run_loop(rx, Duration::from_secs(10));
                if outcome == ShutdownOutcome::Complete {
                    log::info!("Event bus shutdown complete");
                } else {
                    log::error!("Event bus shutdown incomplete: {outcome:?}");
                }
            })
            .expect("failed to spawn event bus thread");
        Self {
            writer,
            events: receiver,
            library,
            library_changed,
        }
    }
}

fn settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join("settings.json")
}

#[cfg(target_os = "android")]
fn providers() -> Providers {
    android::providers()
}

#[cfg(not(target_os = "android"))]
fn providers() -> Providers {
    Providers::default().with_local(n_music_core::source::LocalProvider)
}

#[cfg(test)]
mod tests;
