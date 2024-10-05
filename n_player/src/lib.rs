use flume::Sender;
use multitag::Tag;
use n_audio::music_track::MusicTrack;
use n_audio::queue::QueuePlayer;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::Path;

pub mod bus_server;
pub mod image;
pub mod runner;
pub mod storage;
pub mod ui;

fn loader_thread(tx: Sender<Message>, tracks: Vec<String>) {
    tracks.par_iter().enumerate().for_each(|(i, track)| {
        if let Ok(music_track) = MusicTrack::new(track) {
            let metadata = music_track.get_meta();
            if let Ok(metadata) = metadata {
                tx.send(Message::Length(i, metadata.time.length))
                    .expect("can't send back loaded times");
                if !metadata.artist.is_empty() {
                    tx.send(Message::Artist(i, metadata.artist))
                        .expect("can't send back artist");
                }
                if !metadata.title.is_empty() {
                    tx.send(Message::Title(i, metadata.title))
                        .expect("can't send back title");
                }
            }
        }
    });
}

pub fn get_image<P: AsRef<Path>>(path: P) -> Vec<u8> {
    if let Ok(tag) = Tag::read_from_path(path) {
        if let Some(album) = tag.get_album_info() {
            if let Some(cover) = album.cover {
                return cover.data;
            }
        }
    }

    vec![]
}

#[derive(Debug)]
pub enum Message {
    Length(usize, f64),
    Artist(usize, String),
    Title(usize, String),
}

#[derive(Debug, Clone)]
pub struct FileTrack {
    title: String,
    artist: String,
    length: f64,
}

impl FileTrack {
    pub fn new(title: String, artist: String, length: f64) -> Self {
        Self {
            title,
            artist,
            length,
        }
    }
}

impl PartialEq<Self> for FileTrack {
    fn eq(&self, other: &Self) -> bool {
        self.title.eq(&other.title)
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
        self.title.cmp(&other.title)
    }
}

#[derive(Clone, Debug, Default)]
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

impl From<Vec<FileTrack>> for FileTracks {
    fn from(value: Vec<FileTrack>) -> Self {
        Self { tracks: value }
    }
}

pub fn add_all_tracks_to_player<P: AsRef<Path> + AsRef<OsStr> + From<String>>(
    player: &mut QueuePlayer,
    path: P,
) {
    let dir = fs::read_dir(path).expect("Can't read files in the chosen directory");
    dir.filter_map(|item| item.ok()).for_each(|file| {
        if file.file_type().unwrap().is_file() {
            let mut p = file.path().to_str().unwrap().to_string();
            p.shrink_to_fit();
            player.add(p).unwrap();
        }
    });
    player.shrink_to_fit();

    player.shuffle();
}

pub fn find_cjk_font() -> Option<String> {
    #[cfg(target_family = "unix")]
    {
        use std::process::Command;
        // linux/macOS command: fc-list
        let output = Command::new("sh").arg("-c").arg("fc-list").output().ok()?;
        let stdout = std::str::from_utf8(&output.stdout).ok()?;
        #[cfg(target_os = "macos")]
        let font_line = stdout
            .lines()
            .find(|line| line.contains("Regular") && line.contains("Hiragino Sans GB"))
            .unwrap_or("/System/Library/Fonts/Hiragino Sans GB.ttc");
        #[cfg(target_os = "linux")]
        let font_line = stdout
            .lines()
            .find(|line| line.contains("Regular") && line.contains("CJK"))
            .unwrap_or("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc");

        let font_path = font_line.split(':').next()?.trim();
        Some(font_path.to_string())
    }
    #[cfg(target_os = "windows")]
    {
        use std::path::PathBuf;
        let font_file = {
            // c:/Windows/Fonts/msyh.ttc
            let mut font_path = PathBuf::from(std::env::var("SystemRoot").ok()?);
            font_path.push("Fonts");
            font_path.push("msyh.ttc");
            font_path.to_str()?.to_string().replace("\\", "/")
        };
        Some(font_file)
    }
}
