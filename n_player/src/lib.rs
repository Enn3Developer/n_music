use bitcode::{Decode, Encode};
use multitag::data::Picture;
use multitag::Tag;
use n_audio::queue::QueuePlayer;
use slint::private_unstable_api::re_exports::ColorScheme;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::Path;

pub mod bus_server;
pub mod image;
pub mod runner;
pub mod storage;

pub fn get_image<P: AsRef<Path> + Debug>(path: P) -> Vec<u8> {
    if let Ok(tag) = Tag::read_from_path(path.as_ref()) {
        if let Some(album) = tag.get_album_info() {
            if let Some(cover) = album.cover {
                return cover.data;
            } else {
                if let Tag::OpusTag { inner } = tag {
                    let cover = inner.pictures().first().cloned().map(Picture::from);
                    if let Some(cover) = cover {
                        return cover.data;
                    }
                } else {
                    eprintln!("not an opus tag {path:?}");
                }
            }
        } else {
            eprintln!("no album for {path:?}");
        }
    }

    vec![]
}

pub async fn add_all_tracks_to_player<P: AsRef<Path> + AsRef<OsStr> + From<String>>(
    player: &mut QueuePlayer,
    path: P,
) {
    let mut dir = tokio::fs::read_dir(path)
        .await
        .expect("Can't read files in the chosen directory");
    let mut paths = vec![];
    while let Ok(Some(file)) = dir.next_entry().await {
        if file.file_type().await.unwrap().is_file() {
            let mut p = file.path().to_str().unwrap().to_string();
            p.shrink_to_fit();
            paths.push(p);
        }
    }
    player.add_all(paths).await.unwrap();
    player.shrink_to_fit();

    player.shuffle();
}

#[derive(Copy, Clone, Debug, Decode, Encode)]
pub struct WindowSize {
    pub width: usize,
    pub height: usize,
}

impl Default for WindowSize {
    fn default() -> Self {
        Self {
            width: 450,
            height: 600,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Decode, Encode)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Into<ColorScheme> for Theme {
    fn into(self) -> ColorScheme {
        match self {
            Theme::System => ColorScheme::Unknown,
            Theme::Light => ColorScheme::Light,
            Theme::Dark => ColorScheme::Dark,
        }
    }
}

impl From<Theme> for String {
    fn from(value: Theme) -> Self {
        match value {
            Theme::System => String::from("System"),
            Theme::Light => String::from("Light"),
            Theme::Dark => String::from("Dark"),
        }
    }
}

impl TryFrom<String> for Theme {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if &value == "System" {
            Ok(Self::System)
        } else if &value == "Light" {
            Ok(Self::Light)
        } else if &value == "Dark" {
            Ok(Self::Dark)
        } else {
            Err(format!("{value} is not a valid theme"))
        }
    }
}
