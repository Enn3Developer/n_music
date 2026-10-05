//! Typed settings sections over a platform-provided storage.
//!
//! The core only cares about values: each owner declares a [`Section`] (a serde struct with a
//! key), and reads/writes it through an [`Options`] handle. How sections are kept is up to the
//! platform's [`SettingsStorage`].

mod storage;

pub use storage::{JsonFileStorage, MemoryStorage};

use crate::library::track::ReplayGainMode;
use crate::queue::LoopStatus;
use crate::source::Locator;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, RwLock, RwLockReadGuard};

/// Where sections live, implemented by the platform. Sections are opaque JSON values to it.
pub trait SettingsStorage: Send + Sync {
    /// The saved value of a section, if any. Called when its [`Options`] is created.
    fn load(&self, key: &str) -> Option<Value>;
    /// Saves a section. Must not block on slow I/O: it is called from the event loop.
    fn store(&self, key: &str, value: Value);
}

/// A group of settings saved under one key. Use `#[serde(default)]` so that added or removed
/// fields keep the rest of the section.
pub trait Section: Serialize + DeserializeOwned + Default + Send + Sync + 'static {
    const KEY: &'static str;
}

/// Shared, typed handle to a section; clones see the same value.
pub struct Options<T: Section> {
    inner: Arc<Inner<T>>,
}

struct Inner<T> {
    value: RwLock<T>,
    storage: Arc<dyn SettingsStorage>,
}

impl<T: Section> Clone for Options<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Section> Options<T> {
    /// Reads the section from `storage`; a missing or invalid section uses its defaults.
    pub fn load(storage: Arc<dyn SettingsStorage>) -> Self {
        let value = match storage.load(T::KEY) {
            Some(value) => serde_json::from_value(value).unwrap_or_else(|error| {
                log::warn!(
                    "Invalid settings section {:?}: {error}; using defaults",
                    T::KEY
                );
                T::default()
            }),
            None => T::default(),
        };
        Self {
            inner: Arc::new(Inner {
                value: RwLock::new(value),
                storage,
            }),
        }
    }

    /// A section kept only in memory, for tests and defaults.
    pub fn in_memory() -> Self {
        Self::load(Arc::new(MemoryStorage::default()))
    }

    pub fn get(&self) -> RwLockReadGuard<'_, T> {
        self.inner.value.read().unwrap()
    }

    /// Changes the section and hands it to the storage.
    pub fn update(&self, change: impl FnOnce(&mut T)) {
        let mut value = self.inner.value.write().unwrap();
        change(&mut value);
        match serde_json::to_value(&*value) {
            Ok(json) => self.inner.storage.store(T::KEY, json),
            Err(error) => log::error!("Could not serialize settings section {:?}: {error}", T::KEY),
        }
    }
}

/// Library roots to scan.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LibrarySettings {
    pub libraries: Vec<Locator>,
}

impl Default for LibrarySettings {
    fn default() -> Self {
        Self {
            libraries: default_library().into_iter().collect(),
        }
    }
}

impl Section for LibrarySettings {
    const KEY: &'static str = "core.library";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackSettings {
    pub volume: f64,
    pub loop_status: LoopStatus,
    pub shuffle: bool,
    pub replay_gain: ReplayGainMode,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            volume: 1.0,
            loop_status: LoopStatus::default(),
            shuffle: false,
            replay_gain: ReplayGainMode::default(),
        }
    }
}

impl Section for PlaybackSettings {
    const KEY: &'static str = "core.playback";
}

/// Copies of streamed tracks, kept on disk as they play.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct StreamCacheSettings {
    pub enabled: bool,
    /// The most the copies may take, in bytes.
    pub limit: u64,
}

impl Default for StreamCacheSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            limit: 1 << 30,
        }
    }
}

impl Section for StreamCacheSettings {
    const KEY: &'static str = "core.stream_cache";
}

/// Android folders must be picked through the Storage Access Framework.
#[cfg(target_os = "android")]
fn default_library() -> Option<Locator> {
    None
}

#[cfg(not(target_os = "android"))]
fn default_library() -> Option<Locator> {
    let user_dirs = directories::UserDirs::new()?;
    let path: std::path::PathBuf = match user_dirs.audio_dir() {
        Some(music_dir) => music_dir.into(),
        None => {
            let path = user_dirs.home_dir().join("Music");
            if let Err(error) = std::fs::create_dir_all(&path) {
                log::warn!(
                    "Could not create default music directory {}: {error}",
                    path.display()
                );
            }
            path
        }
    };
    Some(Locator::Local(path.to_string_lossy().into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Serialize, Deserialize, PartialEq, Debug)]
    #[serde(default)]
    struct Sample {
        a: u32,
        b: String,
    }

    impl Section for Sample {
        const KEY: &'static str = "test.sample";
    }

    #[test]
    fn missing_fields_take_defaults() {
        let storage = Arc::new(MemoryStorage::default());
        storage.store(Sample::KEY, serde_json::json!({ "a": 3, "removed": true }));
        let options = Options::<Sample>::load(storage);
        assert_eq!(
            *options.get(),
            Sample {
                a: 3,
                b: String::new()
            }
        );
    }

    #[test]
    fn invalid_section_uses_defaults_and_updates_store() {
        let storage = Arc::new(MemoryStorage::default());
        storage.store(Sample::KEY, serde_json::json!({ "a": "not a number" }));
        let options = Options::<Sample>::load(storage.clone());
        assert_eq!(*options.get(), Sample::default());
        options.update(|sample| sample.a = 7);
        assert_eq!(storage.load(Sample::KEY).unwrap()["a"], 7);
    }
}
