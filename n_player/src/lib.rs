use serde_derive::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::Path;

use n_audio::queue::QueuePlayer;

pub mod app;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FileTrack {
    name: String,
    duration: u64,
}

impl PartialEq<Self> for FileTrack {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
    }
}

impl PartialOrd<Self> for FileTrack {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Eq for FileTrack {}

impl Ord for FileTrack {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct FileTracks {
    tracks: Vec<FileTrack>,
}

impl Deref for FileTracks {
    type Target = Vec<FileTrack>;

    fn deref(&self) -> &Self::Target {
        &self.tracks
    }
}

impl DerefMut for FileTracks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tracks
    }
}

fn vec_contains(tracks: &FileTracks, name: &String) -> bool {
    for track in tracks.tracks.iter() {
        if &track.name == name {
            return true;
        }
    }

    false
}

pub fn add_all_tracks_to_player<P: AsRef<Path>>(player: &mut QueuePlayer<P>, config: &Config)
where
    P: AsRef<OsStr> + From<String>,
{
    let path = config.path();

    if let Some(path) = path {
        let dir = fs::read_dir(path).expect("Can't read files in the chosen directory");
        dir.filter_map(|item| item.ok()).for_each(|file| {
            player.add(file.path().to_str().unwrap().to_string().into());
        });
    }

    player.shuffle();
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    // music dir path
    path: Option<String>,
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
