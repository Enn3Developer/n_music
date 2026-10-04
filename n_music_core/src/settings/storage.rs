use super::SettingsStorage;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Sections kept in memory only.
#[derive(Default)]
pub struct MemoryStorage(Mutex<Map<String, Value>>);

impl SettingsStorage for MemoryStorage {
    fn load(&self, key: &str) -> Option<Value> {
        self.0.lock().unwrap().get(key).cloned()
    }

    fn store(&self, key: &str, value: Value) {
        self.0.lock().unwrap().insert(key.to_string(), value);
    }
}

/// Bursts of changes (e.g. dragging the volume slider) are written once.
const WRITE_DELAY: Duration = Duration::from_millis(500);

/// All sections in one JSON file, written atomically by a background thread. Sections nobody
/// loaded are written back untouched.
pub struct JsonFileStorage {
    shared: Arc<Shared>,
    signal: Mutex<Option<Sender<()>>>,
    writer: Mutex<Option<JoinHandle<()>>>,
}

struct Shared {
    path: PathBuf,
    sections: Mutex<Map<String, Value>>,
}

impl JsonFileStorage {
    pub fn open(path: PathBuf) -> Self {
        let shared = Arc::new(Shared {
            sections: Mutex::new(read_sections(&path)),
            path,
        });
        let (signal, changes) = mpsc::channel();
        let writer_shared = shared.clone();
        let writer = std::thread::Builder::new()
            .name(String::from("settings writer"))
            .spawn(move || write_loop(&writer_shared, &changes))
            .inspect_err(|error| log::error!("Could not start the settings writer: {error}"))
            .ok();
        Self {
            shared,
            // Without a writer thread, stores are written synchronously.
            signal: Mutex::new(writer.is_some().then_some(signal)),
            writer: Mutex::new(writer),
        }
    }

    /// Writes pending changes now and stops the background writer; later stores are written
    /// synchronously.
    pub fn flush(&self) {
        drop(self.signal.lock().unwrap().take());
        if let Some(writer) = self.writer.lock().unwrap().take() {
            if writer.join().is_err() {
                log::error!("Settings writer panicked");
            }
        }
    }
}

impl SettingsStorage for JsonFileStorage {
    fn load(&self, key: &str) -> Option<Value> {
        self.shared.sections.lock().unwrap().get(key).cloned()
    }

    fn store(&self, key: &str, value: Value) {
        {
            let mut sections = self.shared.sections.lock().unwrap();
            if sections.get(key) == Some(&value) {
                return;
            }
            sections.insert(key.to_string(), value);
        }
        match &*self.signal.lock().unwrap() {
            Some(signal) => {
                let _ = signal.send(());
            }
            None => self.shared.write(),
        }
    }
}

impl Drop for JsonFileStorage {
    fn drop(&mut self) {
        self.flush();
    }
}

impl Shared {
    fn write(&self) {
        let json = serde_json::to_vec_pretty(&*self.sections.lock().unwrap());
        let result = json
            .map_err(std::io::Error::other)
            .and_then(|json| crate::library::write_atomic(&self.path, &json));
        if let Err(error) = result {
            log::error!(
                "Could not save settings to {}: {error}",
                self.path.display()
            );
        }
    }
}

fn write_loop(shared: &Shared, changes: &mpsc::Receiver<()>) {
    while changes.recv().is_ok() {
        let deadline = Instant::now() + WRITE_DELAY;
        let mut open = true;
        loop {
            match changes.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(()) => {}
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => {
                    open = false;
                    break;
                }
            }
        }
        shared.write();
        if !open {
            return;
        }
    }
}

fn read_sections(path: &Path) -> Map<String, Value> {
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                log::warn!("Cannot read settings {}: {error}", path.display());
            }
            return Map::new();
        }
    };
    match serde_json::from_slice::<Map<String, Value>>(&data) {
        Ok(sections) => sections,
        Err(error) => {
            // Keep the broken file around instead of overwriting it on the next save.
            let backup = path.with_extension("json.bak");
            log::warn!(
                "Invalid settings {}: {error}; moved to {}",
                path.display(),
                backup.display()
            );
            if let Err(error) = std::fs::rename(path, &backup) {
                log::warn!("Could not back up invalid settings: {error}");
            }
            Map::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_unknown_sections_and_writes_on_flush() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"other.ui": {"x": 1}}"#).unwrap();
        let storage = JsonFileStorage::open(path.clone());
        storage.store("core.playback", serde_json::json!({ "volume": 0.5 }));
        storage.flush();
        let saved: Map<String, Value> =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["other.ui"]["x"], 1);
        assert_eq!(saved["core.playback"]["volume"], 0.5);
    }

    #[test]
    fn backs_up_invalid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "not json").unwrap();
        let storage = JsonFileStorage::open(path.clone());
        assert!(storage.load("core.playback").is_none());
        assert!(path.with_extension("json.bak").exists());
    }
}
