use crate::library::query::{Filter, PlaylistId, Query, SortKey};
use crate::queue::{ItemId, LoopStatus, QueueEntry};
use crate::source::Locator;
use crate::{Track, TrackTime, WindowSize};
use n_event_bus::Message;

macro_rules! messages {
    ($($msg:ty),+ $(,)?) => {
        $( impl Message for $msg {} )+
    };
}

/// Plays what `query` selects, from `start` (the first track when `None`). With the query of
/// the session that is playing, it only moves to `start`, keeping the order and up next.
pub struct PlayFrom {
    pub query: Query,
    pub start: Option<Locator>,
}
/// Queues tracks to play after the current one, before the rest of the context. `next` puts
/// them before what is queued already.
pub struct Enqueue {
    pub tracks: Vec<Locator>,
    pub next: bool,
}
pub struct RemoveQueued(pub ItemId);
pub struct ClearQueued;
pub struct PlayPrevious;
pub struct PlayNext;
pub struct TogglePause;
pub struct Pause;
pub struct Play;
pub struct OutputDeviceChanged;
pub enum Seek {
    FromUi { position: f64, revision: i32 },
    Absolute(f64),
    Relative(f64),
    ToItem { item: ItemId, position: f64 },
}
pub struct SetVolume(pub f64);
pub struct SetLoopStatus(pub LoopStatus);
pub struct ToggleRepeat;
pub struct SetShuffle(pub bool);
pub struct ToggleShuffle;

pub struct PlaybackChanged(pub bool);
pub struct TrackChanged {
    pub item: ItemId,
    pub locator: Locator,
}
pub struct VolumeChanged(pub f64);
pub struct PositionChanged(pub TrackTime, pub i32, pub bool);
pub struct LoopStatusChanged(pub LoopStatus);
pub struct ShuffleChanged(pub bool);
/// The session in play order: the context with up next spliced in after the current item.
pub struct QueueChanged {
    pub entries: Vec<QueueEntry>,
}
/// A track was listened to (past its middle).
pub struct TrackPlayed {
    pub locator: Locator,
    pub fingerprint: Option<u64>,
}

pub struct CreatePlaylist {
    pub name: String,
    /// Makes it a smart playlist.
    pub rule: Option<Filter>,
    pub sort: Vec<SortKey>,
    pub tracks: Vec<Locator>,
}
pub struct RenamePlaylist {
    pub id: PlaylistId,
    pub name: String,
}
pub struct DeletePlaylist(pub PlaylistId);
/// Turns a playlist into a smart one, changes its rule, or (`None`) back into a plain one.
pub struct SetPlaylistRule {
    pub id: PlaylistId,
    pub rule: Option<Filter>,
}
pub struct SetPlaylistSort {
    pub id: PlaylistId,
    pub sort: Vec<SortKey>,
}
pub struct AddToPlaylist {
    pub id: PlaylistId,
    pub tracks: Vec<Locator>,
}
pub struct RemoveFromPlaylist {
    pub id: PlaylistId,
    pub tracks: Vec<Locator>,
}
/// The playlists, after any change; details are in the [`crate::library::catalog::Library`].
pub struct PlaylistsChanged(pub Vec<PlaylistSummary>);
/// A rejected playlist change, e.g. a rule that refers to its own playlist.
pub struct PlaylistRejected(pub String);

pub struct PlaylistSummary {
    pub id: PlaylistId,
    pub name: String,
    pub smart: bool,
}

pub struct SearchChanged(pub String);

pub struct OpenLink(pub String);
pub struct ScanRequested {
    /// `false` reloads every track's metadata instead of trusting the cache.
    pub check_cache: bool,
}
/// The library's tracks; those not loaded yet are placeholders.
pub struct TracksEnumerated {
    pub tracks: Vec<Track>,
}
/// A track's metadata; `index` is its place in [`TracksEnumerated`].
pub struct TrackMetadataLoaded {
    pub index: usize,
    pub track: Track,
}
pub struct ScanFinished {
    /// The scan ran to the end over every root.
    pub complete: bool,
}

pub struct AppVisibilityChanged(pub bool);
pub struct WindowSizeCaptured(pub WindowSize);

pub struct ThemeChangeRequested(pub i32);
pub struct ToggleSaveWindowSize(pub bool);
pub struct LocaleChangeRequested(pub String);
pub struct PathChangeRequested;

messages!(
    OpenLink,
    AppVisibilityChanged,
    WindowSizeCaptured,
    PlayFrom,
    Enqueue,
    RemoveQueued,
    ClearQueued,
    PlayPrevious,
    PlayNext,
    TogglePause,
    Pause,
    Play,
    OutputDeviceChanged,
    Seek,
    SetVolume,
    SetLoopStatus,
    ToggleRepeat,
    SetShuffle,
    ToggleShuffle,
    PlaybackChanged,
    TrackChanged,
    VolumeChanged,
    PositionChanged,
    LoopStatusChanged,
    ShuffleChanged,
    QueueChanged,
    TrackPlayed,
    CreatePlaylist,
    RenamePlaylist,
    DeletePlaylist,
    SetPlaylistRule,
    SetPlaylistSort,
    AddToPlaylist,
    RemoveFromPlaylist,
    PlaylistsChanged,
    PlaylistRejected,
    SearchChanged,
    ScanRequested,
    TracksEnumerated,
    TrackMetadataLoaded,
    ScanFinished,
    ThemeChangeRequested,
    ToggleSaveWindowSize,
    LocaleChangeRequested,
    PathChangeRequested,
);
