//! What the library knows about a track, as read from the file itself.

use crate::source::Locator;
use std::path::PathBuf;
use std::sync::Arc;

/// Shared, immutable track information: the bus, the UI and the queue all hold the same copy.
pub type Track = Arc<TrackInfo>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReplayGain {
    /// Gain in dB.
    pub track_gain: Option<f32>,
    /// Linear sample peak.
    pub track_peak: Option<f32>,
    pub album_gain: Option<f32>,
    pub album_peak: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackInfo {
    pub locator: Locator,
    /// Never empty: falls back to the file name.
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_number: Option<u32>,
    pub disc_total: Option<u32>,
    pub year: Option<i32>,
    pub genres: Vec<String>,
    /// Duration in seconds; 0 when unknown.
    pub length: f64,
    /// Short codec name, e.g. `opus` or `flac`.
    pub codec: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub bits_per_sample: Option<u32>,
    pub replay_gain: ReplayGain,
    /// Thumbnail of the embedded cover, see [`super::covers`].
    pub cover: Option<PathBuf>,
}

impl TrackInfo {
    /// A track whose metadata is not read (yet): only its file name is known.
    pub fn placeholder(locator: Locator) -> Self {
        Self {
            title: locator.display_name(),
            locator,
            artists: vec![],
            album: None,
            album_artist: None,
            track_number: None,
            track_total: None,
            disc_number: None,
            disc_total: None,
            year: None,
            genres: vec![],
            length: 0.0,
            codec: None,
            sample_rate: None,
            channels: None,
            bits_per_sample: None,
            replay_gain: ReplayGain::default(),
            cover: None,
        }
    }

    /// The artists as a single line, for display.
    pub fn artist(&self) -> String {
        self.artists.join(", ")
    }
}
