use rand::seq::SliceRandom;
use rand::thread_rng;
use std::ffi::OsStr;
use std::ops::{Deref, DerefMut};
use std::path::Path;

use crate::music_track::MusicTrack;
use crate::player::Player;
use crate::{from_path_to_name_without_ext, NError, TrackTime};

pub struct QueuePlayer<P: AsRef<Path>>
where
    P: AsRef<OsStr>,
{
    queue: Vec<P>,
    player: Player,
    index: usize,
}

impl<P: AsRef<Path>> Default for QueuePlayer<P>
where
    P: AsRef<OsStr>,
{
    fn default() -> Self {
        Self::new()
    }
}
impl<P: AsRef<Path>> QueuePlayer<P>
where
    P: AsRef<OsStr>,
{
    pub fn new() -> Self {
        let player = Player::new(1.0, 1.0);

        QueuePlayer {
            queue: vec![],
            player,
            index: usize::MAX - 1,
        }
    }

    #[inline]
    pub fn add(&mut self, path: P) {
        self.queue.push(path);
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
            return from_path_to_name_without_ext(self.queue.first().unwrap());
        }

        from_path_to_name_without_ext(self.queue.get(self.index).unwrap())
    }

    pub fn play(&mut self, index: usize) {
        self.index = index;

        let track = MusicTrack::new(&self.queue[self.index]).unwrap();
        let format = track.get_format();

        self.player.play(format);
    }

    pub fn play_next(&mut self) {
        self.index += 1;

        if self.index >= self.queue.len() {
            self.index = 0;
        }

        let track = MusicTrack::new(&self.queue[self.index]).unwrap();
        let format = track.get_format();

        self.player.play(format);
    }

    pub fn play_previous(&mut self) {
        if self.index == 0 {
            self.index = self.queue.len();
        }

        self.index -= 1;

        let track = MusicTrack::new(&self.queue[self.index]).unwrap();
        let format = track.get_format();

        self.player.play(format);
    }

    pub fn get_index_from_track_name(&self, name: &str) -> Result<usize, NError> {
        for (index, track) in self.queue.iter().enumerate() {
            if from_path_to_name_without_ext(track) == name {
                return Ok(index);
            }
        }

        Err(NError::NoTrack)
    }

    pub fn get_duration_for_track(&self, index: usize) -> TrackTime {
        let track = MusicTrack::new(&self.queue[index]).unwrap();
        track.get_duration()
    }
}

impl<P: AsRef<Path>> Deref for QueuePlayer<P>
where
    P: AsRef<OsStr>,
{
    type Target = Player;

    fn deref(&self) -> &Self::Target {
        &self.player
    }
}

impl<P: AsRef<Path>> DerefMut for QueuePlayer<P>
where
    P: AsRef<OsStr>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.player
    }
}
