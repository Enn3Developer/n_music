use std::error::Error;
use std::ffi::OsStr;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::sync::{Arc, Mutex};

use rand::seq::SliceRandom;
use rand::thread_rng;
use symphonia::core::formats::FormatReader;

use crate::music_track::MusicTrack;
use crate::player::Player;
use crate::{NError, TrackTime};

pub struct QueueTrack {
    format: Arc<Mutex<Box<dyn FormatReader>>>,
    name: String,
}

impl QueueTrack {
    pub fn new(format: Arc<Mutex<Box<dyn FormatReader>>>, name: String) -> Self {
        Self { format, name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct QueuePlayer {
    queue: Vec<QueueTrack>,
    player: Player,
    index: usize,
}

impl QueuePlayer {
    pub fn new() -> Self {
        let player = Player::new(1.0, 1.0);

        QueuePlayer {
            queue: vec![],
            player,
            index: usize::MAX - 1,
        }
    }

    #[inline]
    pub fn add_queue_track(&mut self, track: QueueTrack) {
        self.queue.push(track);
    }

    #[inline]
    pub fn add_format(&mut self, format: Arc<Mutex<Box<dyn FormatReader>>>, name: String) {
        self.add_queue_track(QueueTrack { format, name });
    }

    #[inline]
    pub fn add_track(&mut self, track: MusicTrack) {
        let format = Arc::new(Mutex::new(track.get_format()));
        let name = track.name().to_string();
        self.add_format(format, name);
    }

    #[inline]
    pub fn add<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn Error>>
    where
        P: AsRef<OsStr>,
    {
        let track = MusicTrack::new(path)?;

        self.add_track(track);

        Ok(())
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

    pub fn current_track_name(&self) -> &str {
        if self.index == usize::MAX - 1 {
            return &self.queue.get(0).unwrap().name;
        }

        &self.queue.get(self.index).unwrap().name
    }

    pub fn play(&mut self, index: usize) {
        self.index = index;

        self.player.play(self.queue[self.index].format.clone());
    }

    pub fn play_next(&mut self) {
        self.index += 1;

        if self.index >= self.queue.len() {
            self.index = 0;
        }

        self.player.play(self.queue[self.index].format.clone());
    }

    pub fn play_previous(&mut self) {
        if self.index == 0 {
            self.index = self.queue.len();
        }

        self.index -= 1;

        self.player.play(self.queue[self.index].format.clone());
    }

    pub fn get_index_from_track_name(&self, name: &str) -> Result<usize, NError> {
        for (index, track) in self.queue.iter().enumerate() {
            if track.name == name {
                return Ok(index);
            }
        }

        Err(NError::NoTrack)
    }

    pub fn get_duration_for_track(&self, index: usize) -> TrackTime {
        let tmp = self.queue.get(index).unwrap().format.clone();
        let format = tmp.lock().unwrap();

        let track = format.default_track().expect("Can't load tracks");
        let time_base = track.codec_params.time_base.unwrap();

        let duration = track
            .codec_params
            .n_frames
            .map(|frames| track.codec_params.start_ts + frames)
            .unwrap();
        let time = time_base.calc_time(duration);

        TrackTime {
            ts_secs: 0,
            ts_frac: 0.0,
            dur_secs: time.seconds,
            dur_frac: time.frac,
        }
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
