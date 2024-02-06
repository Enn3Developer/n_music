use crossbeam_queue::ArrayQueue;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use std::error::Error;
use std::fs;
use std::fs::DirEntry;
use std::path::Path;
use std::sync::{Arc, Mutex};

use n_audio::music_track::MusicTrack;
use serde_derive::{Deserialize, Serialize};

use n_audio::queue::{QueuePlayer, QueueTrack};

pub mod app;

pub fn add_all_tracks_to_player(player: &mut QueuePlayer, config: &Config) {
    let path = config.path();
    let mut files: Vec<DirEntry> = vec![];

    if let Some(path) = path {
        let paths = fs::read_dir(path).expect("Can't read files in the chosen directory");
        files = paths.filter_map(|item| item.ok()).collect()
    }

    let mut n = 0;
    let q = Arc::new(Mutex::new(0));

    let queue = ArrayQueue::new(files.len());
    files.par_iter().for_each(|file| {
        if file.metadata().unwrap().is_file()
            && infer::get_from_path(file.path())
                .unwrap()
                .unwrap()
                .mime_type()
                .contains("audio")
        {
            {
                let q = q.clone();
                let mut q = q.lock().unwrap();
                *q += 1;
                println!("Getting track from number {q}");
            }
            let track = MusicTrack::new(file.path()).unwrap();
            let format = Arc::new(Mutex::new(track.get_format()));
            let name = track.name().to_string();
            let queue_track = QueueTrack::new(format, name);
            if let Err(e) = queue.push(queue_track) {
                eprintln!("Can't add track {} to queue", e.name());
            }
        }
    });

    for queue_track in queue {
        n += 1;
        println!("Adding track number {n}");
        player.add_queue_track(queue_track);
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
        if let Some(volume) = self.volume.clone() {
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
