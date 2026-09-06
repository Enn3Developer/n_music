use crate::platform::Platform;
use crate::{FileTrack, Theme, WindowSize};
use bitcode::{Decode, Encode};
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufWriter, Cursor};
use std::path::PathBuf;

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
        if storage_file.exists() && storage_file.is_file() {
            let mut data = vec![];
            if let Ok(_) = zstd::stream::copy_decode(
                File::open(storage_file).unwrap(),
                BufWriter::new(Cursor::new(&mut data)),
            ) {
                if let Ok(storage) = bitcode::decode(&data) {
                    storage
                } else {
                    eprintln!("not encoded");
                    Self::default()
                }
            } else {
                eprintln!("bad file");
                Self::default()
            }
        } else {
            eprintln!("file not found");
            Self::default()
        }
    }

    pub async fn read_saved(platform: &(impl Platform + ?Sized)) -> Self {
        let storage_file = platform.internal_dir().await.join("config");
        tokio::task::spawn_blocking(|| Self::read_from_file(storage_file))
            .await
            .unwrap()
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
                if !path.exists() {
                    std::fs::create_dir(&path).unwrap();
                }
                path
            };
        }
        PathBuf::new()
    }

    pub async fn check_timestamp(&self) -> bool {
        if let Some(saved_timestamp) = &self.timestamp {
            if let Ok(timestamp) = self.timestamp().await {
                return &timestamp == saved_timestamp;
            }
        }
        false
    }

    pub async fn timestamp(&self) -> std::io::Result<u64> {
        let mut hasher = DefaultHasher::default();
        tokio::fs::metadata(&self.path)
            .await?
            .modified()?
            .hash(&mut hasher);
        Ok(hasher.finish())
    }

    pub async fn read_tracks(&self, internal_dir: PathBuf) -> Vec<FileTrack> {
        let tracks_file = internal_dir.join("tracks");

        let path = self.path.clone();
        let timestamp = self.timestamp;
        tokio::task::spawn_blocking(move || {
            if tracks_file.exists() && tracks_file.is_file() {
                let mut data = vec![];
                if let Ok(_) = zstd::stream::copy_decode(
                    File::open(tracks_file).unwrap(),
                    BufWriter::new(Cursor::new(&mut data)),
                ) {
                    if let Ok(cache) = bitcode::decode::<TrackCache>(&data) {
                        if cache.path == path && cache.timestamp == timestamp {
                            cache.tracks
                        } else {
                            vec![]
                        }
                    } else {
                        eprintln!("not encoded");
                        vec![]
                    }
                } else {
                    eprintln!("bad file");
                    vec![]
                }
            } else {
                eprintln!("file not found");
                vec![]
            }
        })
        .await
        .unwrap()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            path: Self::music_dir().to_str().unwrap().to_string(),
            volume: 1.0,
            theme: Theme::default(),
            window_size: WindowSize::default(),
            save_window_size: false,
            locale: None,
            timestamp: None,
        }
    }
}

#[derive(Encode, Decode)]
pub struct TrackCache {
    pub path: String,
    pub timestamp: Option<u64>,
    pub tracks: Vec<FileTrack>,
}
