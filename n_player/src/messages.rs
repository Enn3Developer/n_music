use crate::{FileTrack, TrackData};
use n_audio::queue::LoopStatus;
use n_audio::TrackTime;
use n_event_bus::Message;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use symphonia::core::formats::FormatReader;

macro_rules! messages {
    ($($msg:ty),+ $(,)?) => {
        $( impl Message for $msg {} )+
    };
}

pub struct PlayTrack(pub usize);
pub struct PlayPrevious;
pub struct PlayNext;
pub struct TogglePause;
pub struct Pause;
pub struct Play;
pub enum Seek {
    Absolute(f64),
    Relative(f64),
}
pub struct SetVolume(pub f64);
pub struct SetLoopStatus(pub LoopStatus);

pub struct PlaybackChanged(pub bool);
pub struct TrackChanged {
    pub index: usize,
    pub path: PathBuf,
    pub name: Arc<str>,
}
pub struct VolumeChanged(pub f64);
pub struct PositionChanged(pub TrackTime);
pub struct LoopStatusChanged(pub LoopStatus);

pub struct ViewportChanging;
pub struct SearchChanged(pub String);

pub struct ScanRequested {
    pub check_cache: bool,
}
pub struct TracksEnumerated {
    pub path: String,
    pub names: Vec<String>,
    pub tracks: Vec<TrackData>,
}
pub struct TrackMetadataLoaded {
    pub index: usize,
    pub track: FileTrack,
}
pub struct ScanFinished {
    pub tracks: Option<Vec<FileTrack>>,
}
pub struct QueueReplaced {
    pub path: String,
    pub names: Vec<String>,
}

pub struct TrackLoaded(pub Mutex<Option<Box<dyn FormatReader>>>);

pub struct ThemeChangeRequested(pub i32);
pub struct ToggleSaveWindowSize(pub bool);
pub struct LocaleChangeRequested(pub String);
pub struct PathChangeRequested;

messages!(
    PlayTrack,
    PlayPrevious,
    PlayNext,
    TogglePause,
    Pause,
    Play,
    Seek,
    SetVolume,
    SetLoopStatus,
    PlaybackChanged,
    TrackChanged,
    VolumeChanged,
    PositionChanged,
    LoopStatusChanged,
    ViewportChanging,
    SearchChanged,
    ScanRequested,
    QueueReplaced,
    ThemeChangeRequested,
    ToggleSaveWindowSize,
    LocaleChangeRequested,
    PathChangeRequested,
);
