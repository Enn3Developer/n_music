//! What the core last reported, kept as the bridge passes it on, so the app can ask for it at
//! any time rather than only follow events: the library's sources, where scanning is, the
//! output devices, Telegram and the stream cache.

use n_music_core::settings::OutputDevice;
use n_music_core::source::telegram::{TelegramError, TelegramStatus};
use n_music_core::source::Locator;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::SystemTime;

/// Kept by the bridge on the bus thread, read by the queries on any thread.
#[derive(Clone)]
pub struct Shared(Arc<Mutex<Tracked>>);

impl Default for Shared {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Tracked {
            // The engine starts with a scan.
            scanning: true,
            ..Tracked::default()
        })))
    }
}

impl Shared {
    pub fn lock(&self) -> MutexGuard<'_, Tracked> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[derive(Default)]
pub struct Tracked {
    /// The library's sources as it last reported them; `None` before it did.
    pub roots: Option<Vec<Locator>>,
    /// A scan runs or waits.
    pub scanning: bool,
    /// The sources being scanned or waiting for a scan.
    pub updating: Vec<Locator>,
    /// Tracks the running scan found; 0 until it listed its sources.
    pub found: u64,
    /// Tracks found that are still to be read.
    pub unread: u64,
    /// When the last complete scan of every source finished, in Unix seconds.
    pub updated: Option<f64>,
    /// The output devices as the core last listed them; `None` before it did.
    pub devices: Option<Vec<OutputDevice>>,
    /// The output device chosen, kept to list it while it is not there.
    pub chosen: Option<OutputDevice>,
    /// Where signing in to Telegram is; `None` while the core did not tell.
    pub telegram: Option<TelegramSignInState>,
    pub cache: Option<StreamCacheState>,
}

impl Tracked {
    pub fn scan_progress(&mut self, libraries: &[Locator], found: u64, pending: u64) {
        self.scanning = !libraries.is_empty();
        self.updating = libraries.to_vec();
        self.found = found;
        self.unread = pending;
    }

    pub fn track_read(&mut self) {
        self.unread = self.unread.saturating_sub(1);
    }

    pub fn scan_finished(&mut self, libraries: &[Locator], complete: bool) {
        // Only a scan of every source updates the whole library.
        let whole = self
            .roots
            .as_ref()
            .is_some_and(|roots| roots.iter().all(|root| libraries.contains(root)));
        if complete && whole {
            self.updated = Some(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map_or(0.0, |since| since.as_secs_f64()),
            );
        }
    }

    pub fn scan_status(&self) -> ScanStatus {
        ScanStatus {
            running: self.scanning,
            found: self.found,
            read: self.found.saturating_sub(self.unread),
            updated: self.updated,
            updating: self.updating.clone(),
        }
    }

    /// The devices to choose from: the chosen one first while it is not there, then those
    /// listed. Before the first listing, only the chosen one, as connected.
    pub fn output_choices(&self) -> Vec<OutputChoice> {
        let listed = self.devices.as_deref().unwrap_or_default();
        let missing = self
            .chosen
            .as_ref()
            .filter(|chosen| !listed.iter().any(|device| device.id == chosen.id))
            .map(|device| OutputChoice {
                device: device.clone(),
                connected: self.devices.is_none(),
            });
        missing
            .into_iter()
            .chain(listed.iter().map(|device| OutputChoice {
                device: device.clone(),
                connected: true,
            }))
            .collect()
    }

    /// The device with `id`, from those listed or the one chosen; one never seen is named
    /// after its id.
    pub fn device(&self, id: String) -> OutputDevice {
        self.devices
            .iter()
            .flatten()
            .chain(&self.chosen)
            .find(|device| device.id == id)
            .cloned()
            .unwrap_or(OutputDevice {
                name: id.clone(),
                id,
            })
    }
}

/// Where scanning is, as the Sources page and the sidebar card show it.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ScanStatus {
    /// A scan of some sources runs or waits.
    pub running: bool,
    /// Tracks the running scan found; 0 until it listed its sources.
    pub found: u64,
    /// Tracks found that were read, from their files or the cache.
    pub read: u64,
    /// When the last complete scan of every source finished in this launch, in Unix seconds.
    pub updated: Option<f64>,
    /// The sources being scanned or waiting for a scan.
    pub updating: Vec<Locator>,
}

/// An output device to choose from.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct OutputChoice {
    pub device: OutputDevice,
    /// False for the chosen device while it is not there: "%1 (not connected)".
    pub connected: bool,
}

/// Where signing in to Telegram is, as the last `TelegramStatusChanged` told it.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct TelegramSignInState {
    pub status: TelegramStatus,
    /// A step runs: the next waits for it.
    pub busy: bool,
    /// Why the last step failed.
    pub error: Option<TelegramError>,
}

/// The stream cache's settings and what its copies take, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, uniffi::Record)]
pub struct StreamCacheState {
    pub enabled: bool,
    pub limit: u64,
    pub used: u64,
}
