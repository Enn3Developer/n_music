use std::error::Error;
use std::fs;
use std::fs::DirEntry;
use std::path::Path;

use serde_derive::{Deserialize, Serialize};

use n_audio::queue::QueuePlayer;

pub mod app;

pub fn add_all_tracks_to_player(player: &mut QueuePlayer, config: &Config) {
    let path = config.path();
    let mut files: Vec<DirEntry> = vec![];

    if let Some(path) = path {
        let paths = fs::read_dir(path).expect("Can't read files in the chosen directory");
        files = paths.filter_map(|item| item.ok()).collect()
    }

    for file in &files {
        if file.metadata().unwrap().is_file()
            && infer::get_from_path(file.path())
                .unwrap()
                .unwrap()
                .mime_type()
                .contains("audio")
        {
            player.add(file.path()).unwrap();
        }
    }

    player.shuffle();
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    path: Option<String>,
    // music dir path
    volume: Option<f64>,
}

impl Config {
    pub fn new() -> Self {
        Config {
            path: None,
            volume: None,
        }
    }

    pub fn exists(path_to_config: &str) -> bool {
        Path::new(&path_to_config).exists()
    }

    pub fn load(path_to_config: &str) -> Result<Self, Box<dyn Error>> {
        let file = fs::read_to_string(path_to_config)?;
        Ok(toml::from_str(&file)?)
    }

    pub fn save(&self, path_to_config: &str) -> Result<(), Box<dyn Error>> {
        let content = toml::to_string(self)?;
        fs::write(path_to_config, content)?;
        Ok(())
    }

    pub fn path(&self) -> &Option<String> {
        &self.path
    }

    pub fn set_path(&mut self, path: String) {
        self.path = Some(path);
    }

    pub fn volume_or_default(&self, default: f64) -> f64 {
        if let Some(volume) = self.volume {
            return volume;
        }
        default
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.volume = Some(volume);
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
