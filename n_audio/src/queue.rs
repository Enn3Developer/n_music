use rand::seq::SliceRandom;
use rand::thread_rng;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use crate::music_track::MusicTrack;
use crate::player::Player;
use crate::{remove_ext, NError, TrackTime};

pub struct QueuePlayer {
    queue: Vec<String>,
    path: String,
    player: Player,
    index: usize,
}

impl Default for QueuePlayer {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl QueuePlayer {
    pub fn new(path: String) -> Self {
        let player = Player::new(1.0, 1.0);

        QueuePlayer {
            queue: vec![],
            player,
            index: usize::MAX - 1,
            path,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn set_path(&mut self, path: String) {
        self.path = path;
    }

    pub fn get_path_for_file(&self, i: usize) -> PathBuf {
        PathBuf::from(&self.path).join(&self.queue[i])
    }

    fn strip_absolute_path(path: String) -> String {
        let mut s = path
            .split(std::path::MAIN_SEPARATOR)
            .last()
            .unwrap()
            .to_string();
        s.shrink_to_fit();

        s
    }

    pub fn queue(&self) -> Vec<String> {
        self.queue.clone()
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.queue.shrink_to_fit();
        self.path.shrink_to_fit();
    }

    #[inline]
    pub fn add<P: AsRef<Path>>(&mut self, path: P) {
        self.queue.push(Self::strip_absolute_path(
            path.as_ref().to_str().unwrap().to_string(),
        ));
    }

    #[inline]
    pub fn remove(&mut self, index: usize) {
        self.queue.remove(index);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.queue.clear();
        self.index = usize::MAX - 1;
    }

    #[inline]
    pub fn shuffle(&mut self) {
        self.queue.shuffle(&mut thread_rng());
    }

    pub fn current_track_name(&self) -> String {
        if self.index == usize::MAX - 1 {
            self.queue.first().unwrap().clone()
        } else {
            self.queue.get(self.index).unwrap().clone()
        }
    }

    pub fn play(&mut self) {
        let track = MusicTrack::new(self.get_path_for_file(self.index)).unwrap();
        let format = track.get_format();

        self.player.play(format);
    }

    pub fn play_index(&mut self, index: usize) {
        self.index = index;

        self.play();
    }

    pub fn play_next(&mut self) {
        self.index += 1;

        if self.index >= self.queue.len() {
            self.index = 0;
        }

        self.play();
    }

    pub fn play_previous(&mut self) {
        if self.index == 0 {
            self.index = self.queue.len();
        }

        self.index -= 1;

        self.play();
    }

    pub fn get_index_from_track_name(&self, name: &str) -> Result<usize, NError> {
        for (index, track) in self.queue.iter().enumerate() {
            if remove_ext(track) == name {
                return Ok(index);
            }
        }

        Err(NError::NoTrack)
    }

    pub fn get_length_for_track(&self, index: usize) -> TrackTime {
        let track = MusicTrack::new(&self.queue[index]).unwrap();
        track.get_length()
    }
}

impl Deref for QueuePlayer {
    type Target = Player;

    fn deref(&self) -> &Self::Target {
        &self.player
    }
}

impl DerefMut for QueuePlayer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.player
    }
}
