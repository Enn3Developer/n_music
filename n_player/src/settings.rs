use crate::{FileTrack, Theme, WindowSize};
use bitcode::{Decode, Encode};
use std::fs;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufReader, BufWriter, Cursor};
use std::path::PathBuf;

#[derive(Debug, Decode, Encode)]
pub struct Settings {
    pub path: String,
    pub volume: f64,
    pub theme: Theme,
    pub window_size: WindowSize,
    pub save_window_size: bool,
    pub locale: Option<String>,
    pub timestamp: Option<u64>,
    pub tracks: Vec<FileTrack>,
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

    #[cfg(target_os = "android")]
    pub fn read_saved_android(app: &slint::android::AndroidApp) -> Self {
        let config_dir = Self::app_dir(app);
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

    pub async fn check_timestamp(&self) -> bool {
        if let Some(saved_timestamp) = &self.timestamp {
            let mut hasher = DefaultHasher::default();
            tokio::fs::metadata(&self.path)
                .await
                .unwrap()
                .modified()
                .unwrap()
                .hash(&mut hasher);
            let timestamp = hasher.finish();
            &timestamp == saved_timestamp
        } else {
            false
        }
    }

    pub fn save_timestamp(&mut self) {
        let mut hasher = DefaultHasher::default();
        fs::metadata(&self.path)
            .unwrap()
            .modified()
            .unwrap()
            .hash(&mut hasher);
        let timestamp = hasher.finish();
        self.timestamp = Some(timestamp);
    }

    #[cfg(not(target_os = "android"))]
    pub fn save(&self) {
        self.save_and_compress(Self::app_dir());
    }

    #[cfg(target_os = "android")]
    pub fn save(&self, app: &slint::android::AndroidApp) {
        let config_dir = Self::app_dir(app);
        if !config_dir.exists() {
            fs::create_dir(&config_dir).unwrap();
        }
        self.save_and_compress(config_dir);
    }

    fn save_and_compress(&self, config_dir: PathBuf) {
        let storage_file = config_dir.join("config");
        if storage_file.exists() {
            fs::remove_file(&storage_file).unwrap();
        }
        let data = bitcode::encode(self);
        zstd::stream::copy_encode(
            BufReader::new(Cursor::new(data)),
            File::create(storage_file).unwrap(),
            9,
        )
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
            timestamp: None,
            tracks: vec![],
        }
    }
}
