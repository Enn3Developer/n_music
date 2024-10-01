use flume::Sender;
use n_audio::music_track::MusicTrack;
use n_audio::queue::QueuePlayer;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use serde_derive::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

pub mod app;
#[cfg(target_os = "linux")]
pub mod mpris_server;

fn loader_thread(tx: Sender<Message>, tracks: Vec<PathBuf>) {
    tracks.par_iter().enumerate().for_each(|(i, track)| {
        if let Ok(music_track) = MusicTrack::new(track) {
            let metadata = music_track.get_meta();
            tx.send(Message::Duration(i, metadata.time.dur_secs))
                .expect("can't send back loaded times");
            tx.send(Message::Artist(i, metadata.artist))
                .expect("can't send back artist");
        }
    });
}

enum Message {
    Duration(usize, u64),
    Artist(usize, String),
    // Image(usize, Vec<u8>),
}

#[derive(Debug)]
pub enum ServerMessage {
    PlayNext,
    PlayPrevious,
    TogglePause,
    Pause,
    Play,
    SetVolume(f64),
    AskVolume,
    AskPlayback,
    AskMetadata,
    AskTime,
}

#[derive(Debug)]
pub enum ClientMessage {
    Volume(f64),
    Playback(bool),
    Metadata(Option<String>, Option<Vec<String>>, u64, String),
    Time(u64),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileTrack {
    name: String,
    artist: String,
    duration: u64,
    cover: Vec<u8>,
}

impl PartialEq<Self> for FileTrack {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
    }
}

impl PartialOrd<Self> for FileTrack {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for FileTrack {}

impl Ord for FileTrack {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileTracks {
    pub tracks: Vec<FileTrack>,
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

fn vec_contains(tracks: &FileTracks, name: &String) -> (bool, usize) {
    for (i, track) in tracks.tracks.iter().enumerate() {
        if &track.name == name {
            return (true, i);
        }
    }

    (false, 0)
}

pub fn add_all_tracks_to_player<P: AsRef<Path>>(player: &mut QueuePlayer, path: P)
where
    P: AsRef<OsStr> + From<String>,
{
    let dir = fs::read_dir(path).expect("Can't read files in the chosen directory");
    dir.filter_map(|item| item.ok()).for_each(|file| {
        let mut p = file.path().to_str().unwrap().to_string();
        p.shrink_to_fit();
        player.add(p.to_string());
    });
    player.shrink_to_fit();

    player.shuffle();
}
