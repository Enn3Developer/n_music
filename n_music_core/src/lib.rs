mod audio;
pub mod engine;
pub mod library;
pub mod logging;
pub mod messages;
pub mod queue;
pub mod settings;
pub mod source;

pub use library::track::{Track, TrackInfo};

/// Where playback is in a track, in seconds.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct TrackTime {
    pub position: f64,
    pub length: f64,
}
