use symphonia::core::codecs::registry::CodecRegistry;

use crate::dca::DcaReader;
use crate::opus::OpusDecoder;
use crate::raw::RawReader;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use symphonia::default::{register_enabled_codecs, register_enabled_formats};
use symphonia_core::formats::probe::Probe;

mod dca;
pub mod jobs;
pub mod library;
pub mod logging;
pub mod messages;
pub mod music_track;
mod opus;
mod output;
pub mod platform;
pub mod player;
pub mod queue;
mod raw;
pub mod services;
pub mod settings;
pub mod source;

pub use library::track::{Track, TrackInfo};

/// Default Symphonia [`CodecRegistry`], including the (audiopus-backed) Opus codec.
pub static CODEC_REGISTRY: Lazy<CodecRegistry> = Lazy::new(|| {
    let mut registry = CodecRegistry::new();
    register_enabled_codecs(&mut registry);
    registry.register_audio_decoder::<OpusDecoder>();
    registry
});

pub static PROBE: Lazy<Probe> = Lazy::new(|| {
    let mut probe = Probe::default();
    probe.register_format::<DcaReader>();
    probe.register_format::<RawReader>();
    register_enabled_formats(&mut probe);
    probe
});

#[derive(Debug)]
pub enum NError {
    NoTrack,
}

/// Used to represent the timestamp
///
/// pos_* is used to represent the *current* timestamp (as in where is currently the player playing inside the track)
/// len_* is used to represent the *entire* timestamp (as is how long is the track)
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct TrackTime {
    pub position: f64,
    pub length: f64,
}

impl TrackTime {
    pub fn format_pos(&self) -> String {
        format!(
            "{:02}:{:02}",
            (self.position / 60.0).floor() as u64,
            self.position.floor() as u64 % 60
        )
    }

    pub fn format_len(&self) -> String {
        format!(
            "{:02}:{:02}",
            (self.length / 60.0).floor() as u64,
            self.length.floor() as u64 % 60
        )
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: usize,
    pub height: usize,
}

impl Default for WindowSize {
    fn default() -> Self {
        Self {
            width: 450,
            height: 625,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
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
impl From<Theme> for i32 {
    fn from(value: Theme) -> Self {
        match value {
            Theme::System => 0,
            Theme::Light => 1,
            Theme::Dark => 2,
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

impl TryFrom<i32> for Theme {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value == 0 {
            Ok(Self::System)
        } else if value == 1 {
            Ok(Self::Light)
        } else if value == 2 {
            Ok(Self::Dark)
        } else {
            Err(format!("{value} is not a valid theme"))
        }
    }
}
