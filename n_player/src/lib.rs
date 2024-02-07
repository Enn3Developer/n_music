use n_audio::music_track::MusicTrack;
use serde_derive::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use n_audio::queue::QueuePlayer;

pub mod app;

fn loader_thread(tx: Sender<Message>, tracks: Vec<PathBuf>) {
    for (i, track) in tracks.iter().enumerate() {
        if let Ok(music_track) = MusicTrack::new(&track) {
            let duration = music_track.get_duration();
            tx.send(Message::Duration(i, duration.dur_secs))
                .expect("can't send back loaded times");
        }
    }
}

enum Message {
    Duration(usize, u64),
    Image, // TODO
}

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

fn vec_contains(tracks: &FileTracks, name: &String) -> (bool, usize) {
    for (i, track) in tracks.tracks.iter().enumerate() {
        if &track.name == name {
            return (true, i);
        }
    }

    (false, 0)
}

pub fn add_all_tracks_to_player<P: AsRef<Path>>(player: &mut QueuePlayer<P>, path: P)
where
    P: AsRef<OsStr> + From<String>,
{
    let dir = fs::read_dir(path).expect("Can't read files in the chosen directory");
    dir.filter_map(|item| item.ok()).for_each(|file| {
        player.add(file.path().to_str().unwrap().to_string().into());
    });

    player.shuffle();
}
