use std::path::PathBuf;
use std::sync::Arc;

/// Repeat mode as understood by the media backends.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum LoopStatus {
    #[default]
    Playlist,
    File,
}

/// Everything a backend needs to render the current media state.
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub playing: bool,
    pub volume: f64,
    pub loop_status: LoopStatus,
    pub position: f64,
    pub length: f64,
    pub title: String,
    pub artist: String,
    pub cover: Option<PathBuf>,
}

/// Which part of [`State`] just changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Change {
    Playback,
    Volume,
    Loop,
    Position { discontinuity: bool },
    Metadata,
}

/// A control event coming from the OS media controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MediaEvent {
    Play,
    Pause,
    TogglePause,
    Next,
    Previous,
    SeekRelative(f64),
    SeekAbsolute(f64),
    SetVolume(f64),
    SetLoopStatus(LoopStatus),
}

pub(crate) type Emit = Arc<dyn Fn(MediaEvent) + Send + Sync + 'static>;

pub(crate) trait Backend: Send + Sync {
    fn update(&self, state: &State, change: Change);
}
