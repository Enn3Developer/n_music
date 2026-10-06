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
use n_event_bus::{App, EventReceiver, EventWriter, JobControl, ShutdownOutcome};
use n_music_core::engine::Engine;
use n_music_core::library::catalog::Library;
use n_music_core::library::query::Query;
use n_music_core::settings::{JsonFileStorage, SettingsStorage};
use n_music_core::source::Providers;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

uniffi::setup_scaffolding!();

static CORE: OnceLock<Arc<Core>> = OnceLock::new();

/// The core services on their bus, for the whole life of the process.
#[derive(uniffi::Object)]
pub struct Core {
    writer: EventWriter,
    events: flume::Receiver<CoreEvent>,
    /// Set once the services started on the bus thread; queries find nothing until then.
    library: Arc<OnceLock<Library>>,
    library_changed: Arc<AtomicBool>,
    scan_read: Arc<AtomicU64>,
    storage: Arc<JsonFileStorage>,
}

#[uniffi::export]
impl Core {
    /// Starts the core, once per process: later calls return the core already running.
    ///
    /// `data_dir` keeps what the user creates (settings, the library database, logs);
    /// `cache_dir` what can be rebuilt (covers, copies of streamed tracks). On Android,
    /// `NativeLibrary.init` must have run first.
    ///
    /// Only the settings are read before this returns. The services open the library database
    /// and the saved session on the bus thread, so the main thread can call this; commands sent
    /// meanwhile wait for them.
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
    ///
    /// The bus thread wakes the waiting caller, so await this on a dispatcher that resumes on
    /// another thread, like `Dispatchers.Default`. One that resumes in place would poll again on
    /// the bus thread while it holds the queue's lock, and the core would hang.
    pub async fn next_event(&self) -> Option<CoreEvent> {
        let event = self.events.recv_async().await.ok()?;
        Some(self.taken(event))
    }

    /// The tracks `query` selects, in its order: `limit` of them from `offset`.
    pub fn tracks(&self, query: Query, offset: u32, limit: u32) -> Vec<TrackRow> {
        let Some(library) = self.library.get() else {
            return vec![];
        };
        // A scan waits on the lock to add each track it reads: hold it only to select.
        let selected = library.read().select(&query);
        selected
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|track| TrackRow::from(&**track))
            .collect()
    }

    /// How many tracks `query` selects.
    pub fn count(&self, query: Query) -> u32 {
        let Some(library) = self.library.get() else {
            return 0;
        };
        let count = library.read().select(&query).len();
        u32::try_from(count).unwrap_or(u32::MAX)
    }

    /// The settings section `key` as JSON, or `None` when nothing is stored under it. The
    /// interface keeps its own sections next to the core's, in the same file.
    pub fn setting(&self, key: String) -> Option<String> {
        self.storage.load(&key).map(|value| value.to_string())
    }

    /// Stores `json` as the settings section `key`; text that is not JSON is dropped.
    pub fn set_setting(&self, key: String, json: String) {
        match serde_json::from_str(&json) {
            Ok(value) => self.storage.store(&key, value),
            Err(error) => log::error!("Not storing settings section {key}: {error}"),
        }
    }
}

impl Core {
    /// Notes that Kotlin took `event`.
    fn taken(&self, event: CoreEvent) -> CoreEvent {
        match event {
            CoreEvent::LibraryChanged { .. } => {
                // Changes from now on announce themselves again.
                self.library_changed.store(false, Ordering::Release);
                CoreEvent::LibraryChanged {
                    read: self.scan_read.load(Ordering::Acquire),
                }
            }
            event => event,
        }
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
        // Read here, so the interface's own sections are there as soon as this returns.
        let storage = Arc::new(JsonFileStorage::open(settings_path(data_dir)));

        let (writer, rx) = EventWriter::channel();
        let (events, receiver) = flume::unbounded();
        let library = Arc::new(OnceLock::new());
        let library_changed = Arc::new(AtomicBool::new(false));
        let scan_read = Arc::new(AtomicU64::new(0));
        let services = Services {
            writer: writer.clone(),
            storage: storage.clone(),
            providers: providers(),
            data_dir: data_dir.to_path_buf(),
            cache_dir: cache_dir.to_path_buf(),
            library: library.clone(),
            events,
            library_changed: library_changed.clone(),
            scan_read: scan_read.clone(),
        };
        std::thread::Builder::new()
            .name(String::from("n_event_bus loop"))
            .spawn(move || services.run(rx))
            .expect("failed to spawn event bus thread");
        Self {
            writer,
            events: receiver,
            library,
            library_changed,
            scan_read,
            storage,
        }
    }
}

/// What the bus thread starts the core services with.
struct Services {
    writer: EventWriter,
    storage: Arc<JsonFileStorage>,
    providers: Providers,
    data_dir: PathBuf,
    cache_dir: PathBuf,
    library: Arc<OnceLock<Library>>,
    events: flume::Sender<CoreEvent>,
    library_changed: Arc<AtomicBool>,
    scan_read: Arc<AtomicU64>,
}

impl Services {
    /// Starts the services and runs the bus until shutdown, on the bus thread.
    fn run(self, rx: EventReceiver) {
        let run = std::panic::catch_unwind(AssertUnwindSafe(|| {
            // The bus thread wakes Kotlin for each event it queues. Attached once, it is not
            // attached and detached again for every one.
            #[cfg(target_os = "android")]
            android::attach_current_thread();
            let mut app = App::new(JobControl::new(self.writer.clone()));
            let engine = Engine::start(
                &mut app,
                &self.writer,
                self.storage,
                self.providers,
                &self.data_dir,
                &self.cache_dir,
            );
            let library = engine.library();
            let _ = self.library.set(library.clone());
            // After the core services, so the library is up to date when Kotlin hears.
            app.register_subscriber(KotlinBridge::new(
                self.events,
                library,
                self.library_changed,
                self.scan_read,
            ));
            app.run_loop(rx, Duration::from_secs(10))
        }));
        match run {
            Ok(ShutdownOutcome::Complete) => log::info!("Event bus shutdown complete"),
            Ok(outcome) => log::error!("Event bus shutdown incomplete: {outcome:?}"),
            Err(_) => {
                // The panic hook logged why. Carrying on would leave Kotlin with a core that
                // is gone: the screens and the notification stuck as they were, the wake lock
                // held, every command lost. Ending the process lets Android start it afresh.
                log::error!("The event bus panicked; ending the process");
                log::logger().flush();
                std::process::abort();
            }
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
