//! What the library knows about a track, as read from the file itself.

use crate::source::Locator;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use symphonia_core::meta::StandardTag;

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

impl ReplayGain {
    /// Takes the first value of a ReplayGain tag; returns whether `tag` is one.
    pub(crate) fn read(&mut self, tag: &StandardTag) -> bool {
        let (field, value) = match tag {
            StandardTag::ReplayGainTrackGain(value) => (&mut self.track_gain, value),
            StandardTag::ReplayGainTrackPeak(value) => (&mut self.track_peak, value),
            StandardTag::ReplayGainAlbumGain(value) => (&mut self.album_gain, value),
            StandardTag::ReplayGainAlbumPeak(value) => (&mut self.album_peak, value),
            _ => return false,
        };
        // Gains are written like `-6.5 dB`.
        let value = value
            .trim()
            .trim_end_matches(|c: char| c.is_alphabetic() || c.is_whitespace())
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite());
        if let Some(value) = value {
            field.get_or_insert(value);
        }
        true
    }

    /// The linear gain to apply in `mode`, lowered when the peak would clip; 1 when untagged.
    pub fn factor(&self, mode: ReplayGainMode) -> f32 {
        let (gain, peak) = match mode {
            ReplayGainMode::Off => return 1.0,
            ReplayGainMode::Track => (self.track_gain, self.track_peak),
            ReplayGainMode::Album => match self.album_gain {
                Some(gain) => (Some(gain), self.album_peak),
                None => (self.track_gain, self.track_peak),
            },
        };
        let Some(gain) = gain else {
            return 1.0;
        };
        let factor = 10f32.powf(gain / 20.0);
        match peak.filter(|peak| *peak > 0.0) {
            Some(peak) => factor.min(1.0 / peak),
            None => factor,
        }
    }
}

/// Which ReplayGain value playback applies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayGainMode {
    #[default]
    Off,
    Track,
    /// Album gain, or the track gain when the album has none.
    Album,
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
    /// See [`super::fingerprint`]; `None` when there was no audio to hash.
    pub fingerprint: Option<u64>,
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
            fingerprint: None,
        }
    }

    /// The artists as a single line, for display.
    pub fn artist(&self) -> String {
        self.artists.join(", ")
    }
}
