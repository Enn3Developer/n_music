use std::path::Path;

use symphonia::core::units::Time;

mod music_track;
mod output;
pub mod player;

#[derive(Debug)]
pub enum NError {
    NoTrack,
}

pub enum Message {
    Play,
    Pause,
    End,
    Exit,
    Seek(Time),
    Time(TrackTime),
    Volume(f32),
}

pub fn from_path_to_name_without_ext(path: &Path) -> String {
    let split: Vec<String> = path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .split('.')
        .map(String::from)
        .collect();
    split[..split.len() - 1].to_vec().join(".")
}

#[derive(Clone)]
pub struct TrackTime {
    pub ts_secs: u64,
    pub ts_frac: f64,
    pub dur_secs: u64,
    pub dur_frac: f64,
}
