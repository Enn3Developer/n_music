use std::path::Path;
use symphonia::core::codecs::CodecRegistry;

use crate::dca::DcaReader;
use crate::opus::OpusDecoder;
use crate::raw::RawReader;
use once_cell::sync::Lazy;
use symphonia::core::units::Time;
use symphonia::default::{register_enabled_codecs, register_enabled_formats};
use symphonia_core::probe::Probe;

mod dca;
pub mod music_track;
mod opus;
mod output;
pub mod player;
pub mod queue;
mod raw;

/// Default Symphonia [`CodecRegistry`], including the (audiopus-backed) Opus codec.
pub static CODEC_REGISTRY: Lazy<CodecRegistry> = Lazy::new(|| {
    let mut registry = CodecRegistry::new();
    register_enabled_codecs(&mut registry);
    registry.register_all::<OpusDecoder>();
    registry
});

pub static PROBE: Lazy<Probe> = Lazy::new(|| {
    let mut probe = Probe::default();
    probe.register_all::<DcaReader>();
    probe.register_all::<RawReader>();
    register_enabled_formats(&mut probe);
    probe
});

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
/// use n_audio::remove_ext;
/// let filename = "file.1.txt";
/// assert_eq!(remove_ext(filename), "file.1");
/// ```
pub fn remove_ext<P: AsRef<Path>>(path: P) -> String {
    let split: Vec<String> = path
        .as_ref()
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
/// pos_* is used to represent the *current* timestamp (as in where is currently the player playing inside the track)
/// len_* is used to represent the *entire* timestamp (as is how long is the track)
#[derive(Copy, Clone, Debug, Default)]
pub struct TrackTime {
    pub pos_secs: u64,
    pub pos_frac: f64,
    pub len_secs: u64,
    pub len_frac: f64,
}

#[derive(Clone, Debug)]
pub struct Metadata {
    pub time: TrackTime,
    pub artist: String,
}
