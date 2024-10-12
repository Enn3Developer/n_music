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
    fn read_from_file(storage_file: PathBuf) -> Self {
        if storage_file.exists() && storage_file.is_file() {
            let storage_content = fs::read(storage_file).unwrap();
            if let Ok(storage) = bitcode::decode(&storage_content) {
                storage
            } else {
                Self::default()
            }
        } else {
            Self::default()
        }
    }

    #[cfg(target_os = "android")]
    pub fn read_saved_android(app: slint::android::AndroidApp) -> Self {
        let data_path = app
            .external_data_path()
            .expect("can't get external data path");
        let config_dir = data_path.join("config/");
        if !config_dir.exists() {
            fs::create_dir(&config_dir).unwrap();
        }
        let storage_file = config_dir.join("config");
        Self::read_from_file(storage_file)
    }

    #[cfg(not(target_os = "android"))]
    pub async fn read_saved() -> Self {
        let storage_file = Self::app_dir().join("config");

        tokio::task::spawn_blocking(|| Self::read_from_file(storage_file))
            .await
            .unwrap()
    }

    #[cfg(target_os = "android")]
    pub fn app_dir(app: &slint::android::AndroidApp) -> PathBuf {
        app.external_data_path()
            .expect("can't get external data path")
            .join("config/")
    }

    #[cfg(not(target_os = "android"))]
    pub fn app_dir() -> PathBuf {
        let base_dirs = directories::BaseDirs::new().unwrap();
        let local_data_dir = base_dirs.data_local_dir();
        let app_dir = local_data_dir.join("n_music");
        if !app_dir.exists() {
            fs::create_dir(app_dir.as_path()).unwrap();
        }
        app_dir
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
                    fs::create_dir(&path).unwrap();
                }
                path
            };
        }
        PathBuf::new()
    }

    #[cfg(not(target_os = "android"))]
    pub async fn save(&self) {
        let storage_file = Self::app_dir().join("config");
        tokio::fs::write(storage_file, bitcode::encode(self))
            .await
            .unwrap();
    }
    #[cfg(target_os = "android")]
    pub async fn save(&self, app: &slint::android::AndroidApp) {
        let config_dir = Self::app_dir(app).join("config/");
        if !config_dir.exists() {
            tokio::fs::create_dir(&config_dir).await.unwrap();
        }
        let storage_file = config_dir.join("config");
        tokio::fs::write(storage_file, bitcode::encode(self))
            .await
            .unwrap();
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
