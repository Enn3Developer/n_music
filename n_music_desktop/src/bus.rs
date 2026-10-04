//! The event bus with the core services, running beside the Qt event loop.

use crate::hub;
use crate::listener::Listener;
use n_event_bus::{App, EventWriter, JobControl, Message, ShutdownOutcome};
use n_music_core::engine::Engine;
use n_music_core::settings::SettingsStorage;
use n_music_core::source::{LocalProvider, Providers};
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;

static WRITER: OnceLock<EventWriter> = OnceLock::new();

/// Sends `message` to the core.
pub fn emit<T: Message>(message: T) {
    if let Some(writer) = WRITER.get() {
        writer.emit(message);
    }
}

pub struct Bus {
    thread: JoinHandle<()>,
    writer: EventWriter,
}

impl Bus {
    /// Starts the core services on a bus of their own thread.
    ///
    /// `data_dir` keeps what the user creates, `cache_dir` what can be rebuilt.
    pub fn start(storage: Arc<dyn SettingsStorage>, data_dir: &Path, cache_dir: &Path) -> Self {
        let (writer, rx) = EventWriter::channel();
        let _ = WRITER.set(writer.clone());
        let mut app = App::new(JobControl::new(writer.clone()));
        let providers = Arc::new(Providers::default().with_local(LocalProvider));
        let engine = Engine::start(&mut app, &writer, storage, providers, data_dir, cache_dir);
        hub::init(engine.library());
        // After the core services, so the library is up to date when the interface hears.
        app.register_subscriber(Listener);
        let thread = std::thread::Builder::new()
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
        Self { thread, writer }
    }

    /// Asks the services to shut down and waits for them.
    pub fn stop(self) {
        log::debug!("Shutdown requested");
        self.writer.shutdown();
        if self.thread.join().is_err() {
            log::error!("Event bus thread panicked");
        }
    }
}
