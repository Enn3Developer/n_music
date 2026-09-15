use crate::platform::Platform;
use crate::{FileTrack, Theme, WindowSize};
use bitcode::{Decode, Encode};
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Decode, Encode)]
pub struct Settings {
    pub path: String,
    pub volume: f64,
    pub theme: Theme,
    pub window_size: WindowSize,
    pub save_window_size: bool,
    pub locale: Option<String>,
    pub timestamp: Option<u64>,
}

impl Settings {
    fn read_from_file(storage_file: PathBuf) -> Self {
        let Some(data) = read_compressed_file(&storage_file) else {
            return Self::default();
        };
        match bitcode::decode(&data) {
            Ok(settings) => settings,
            Err(error) => {
                log::warn!(
                    "Invalid settings in {}: {error}; using defaults",
                    storage_file.display()
                );
                Self::default()
            }
        }
    }

    pub fn read_saved(platform: &(impl Platform + ?Sized)) -> Self {
        let storage_file = platform.internal_dir().join("config");
        Self::read_from_file(storage_file)
    }

    #[cfg(target_os = "android")]
    pub fn music_dir() -> PathBuf {
        PathBuf::new()
    }

    #[cfg(not(target_os = "android"))]
    pub fn music_dir() -> PathBuf {
        if let Some(user_dirs) = directories::UserDirs::new() {
            return if let Some(music_dir) = user_dirs.audio_dir() {
                music_dir.into()
            } else {
                let path = user_dirs.home_dir().join("Music");
                if let Err(error) = std::fs::create_dir_all(&path) {
                    log::warn!(
                        "Could not create default music directory {}: {error}",
                        path.display()
                    );
                }
                path
            };
        }
        PathBuf::new()
    }

    pub fn check_timestamp(&self) -> bool {
        if let Some(saved_timestamp) = &self.timestamp {
            if let Ok(timestamp) = self.timestamp() {
                return &timestamp == saved_timestamp;
            }
        }
        false
    }

    pub fn timestamp(&self) -> std::io::Result<u64> {
        let mut hasher = DefaultHasher::default();
        std::fs::metadata(&self.path)?.modified()?.hash(&mut hasher);
        Ok(hasher.finish())
    }

    pub fn read_tracks(&self, internal_dir: PathBuf) -> Vec<FileTrack> {
        let tracks_file = internal_dir.join("tracks");
        let Some(data) = read_compressed_file(&tracks_file) else {
            return vec![];
        };
        match bitcode::decode::<TrackCache>(&data) {
            Ok(cache) if cache.path == self.path && cache.timestamp == self.timestamp => {
                cache.tracks
            }
            Ok(_) => {
                log::debug!("Ignoring stale track cache in {}", tracks_file.display());
                vec![]
            }
            Err(error) => {
                log::warn!("Invalid track cache in {}: {error}", tracks_file.display());
                vec![]
            }
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            path: Self::music_dir().to_string_lossy().into_owned(),
            volume: 1.0,
            theme: Theme::default(),
            window_size: WindowSize::default(),
            save_window_size: false,
            locale: None,
            timestamp: None,
        }
    }
}

fn read_compressed_file(path: &Path) -> Option<Vec<u8>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                log::debug!("No saved file at {}", path.display());
            } else {
                log::warn!("Cannot open {}: {error}", path.display());
            }
            return None;
        }
    };
    match zstd::stream::decode_all(file) {
        Ok(data) => Some(data),
        Err(error) => {
            log::warn!("Cannot decompress {}: {error}", path.display());
            None
        }
    }
}

#[derive(Encode, Decode)]
pub struct TrackCache {
    pub path: String,
    pub timestamp: Option<u64>,
    pub tracks: Vec<FileTrack>,
}
