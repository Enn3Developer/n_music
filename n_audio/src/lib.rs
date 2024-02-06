use std::path::Path;

use symphonia::core::units::Time;

pub mod music_track;
mod output;
pub mod player;
pub mod queue;

#[derive(Debug)]
pub enum NError {
    NoTrack,
}

/// Messages sent inside the `Player`
pub enum Message {
    Play,
    Pause,
    End,
    Exit,
    Seek(Time),
    Time(TrackTime),
    Volume(f32),
    PlaybackSpeed(f32),
}

/// Returns the file name without its extension
///
/// # Example
/// ```
/// use std::path::Path;
/// use n_audio::from_path_to_name_without_ext;
/// let filename = "file.1.txt";
/// assert_eq!(from_path_to_name_without_ext(Path::new(filename)), "file.1");
/// ```
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

/// Used to represent the timestamp
/// ts_* is used to represent the *current* timestamp (as in where is currently the player playing inside the track)
/// dur_* is used to represent the *entire* timestamp (as is how long is the track)
#[derive(Clone, Debug)]
pub struct TrackTime {
    pub ts_secs: u64,
    pub ts_frac: f64,
    pub dur_secs: u64,
    pub dur_frac: f64,
}
