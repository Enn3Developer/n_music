use crate::{Theme, WindowSize};
use bitcode::{Decode, Encode};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Decode, Encode)]
pub struct Settings {
    pub path: String,
    pub volume: f64,
    pub theme: Theme,
    pub window_size: WindowSize,
    pub save_window_size: bool,
    pub locale: Option<String>,
}

impl Settings {
    pub async fn read_saved() -> Self {
        let storage_file = if cfg!(not(target_os = "android")) {
            Self::app_dir().join("config")
        } else {
            PathBuf::new()
        };
        if storage_file.exists() && storage_file.is_file() {
            let storage_content = tokio::fs::read(storage_file).await.unwrap();
            if let Ok(storage) = bitcode::decode(&storage_content) {
                storage
            } else {
                Self::default()
            }
        } else {
            Self::default()
        }
    }

    pub fn app_dir() -> PathBuf {
        let base_dirs = directories::BaseDirs::new().unwrap();
        let local_data_dir = base_dirs.data_local_dir();
        let app_dir = local_data_dir.join("n_music");
        if !app_dir.exists() {
            fs::create_dir(app_dir.as_path()).unwrap();
        }
        app_dir
    }

    pub fn music_dir() -> PathBuf {
        if let Some(user_dirs) = directories::UserDirs::new() {
            return if let Some(music_dir) = user_dirs.audio_dir() {
                music_dir.into()
            } else {
                let path = user_dirs.home_dir().join("Music");
                if !path.exists() {
                    fs::create_dir(&path).unwrap();
                }
                path
            };
        }
        PathBuf::new()
    }

    pub async fn save(&self) {
        if cfg!(not(target_os = "android")) {
            let storage_file = Self::app_dir().join("config");
            tokio::fs::write(storage_file, bitcode::encode(self))
                .await
                .unwrap();
        }
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
        }
    }
}
