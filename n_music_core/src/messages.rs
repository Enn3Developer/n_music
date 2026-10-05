use crate::library::query::{Filter, PlaylistId, Query, SortKey};
use crate::library::track::ReplayGainMode;
use crate::queue::{ItemId, LoopStatus, QueueEntry};
use crate::settings::OutputDevice;
use crate::source::Locator;
use crate::{Track, TrackTime};
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
/// The system's audio devices changed, e.g. headphones were plugged in: playback reopens on
/// the device it should be on.
pub struct OutputDeviceChanged;
/// Plays on this device while it is there, or (`None`) on the system default.
pub struct SetOutputDevice(pub Option<OutputDevice>);
/// Asks for the devices there are to play on, as [`OutputDevices`].
pub struct ListOutputDevices;
/// The devices there are to play on.
pub struct OutputDevices(pub Vec<OutputDevice>);
pub enum Seek {
    Absolute(f64),
    Relative(f64),
    /// An absolute seek whose result can be told apart: positions report `request` as
    /// [`PositionChanged::seek`] once it applied, so a UI dragging a slider can ignore
    /// positions sent before.
    Tracked {
        position: f64,
        request: u64,
    },
    ToItem {
        item: ItemId,
        position: f64,
    },
}
pub struct SetVolume(pub f64);
pub struct SetLoopStatus(pub LoopStatus);
pub struct ToggleRepeat;
pub struct SetShuffle(pub bool);
pub struct ToggleShuffle;
/// Takes effect from the next track.
pub struct SetReplayGain(pub ReplayGainMode);
/// Keeps the play session between launches, reopening it on the next one; off forgets the one
/// kept.
pub struct SetResume(pub bool);
/// Fades each track into the next over this many seconds; 0 plays them back to back, without a
/// gap. Applies from the next change of track.
pub struct SetCrossfade(pub f64);

pub struct PlaybackChanged(pub bool);
/// The current item, with what the library knows about its track. Sent again when the
/// track's metadata loads while it is current.
pub struct TrackChanged {
    pub item: ItemId,
    pub track: Track,
}
pub struct VolumeChanged(pub f64);
pub struct PositionChanged {
    pub time: TrackTime,
    /// The last [`Seek::Tracked`] request that applied, 0 before any.
    pub seek: u64,
    /// The position jumped (seek, new track, pause) rather than advanced with playback.
    pub discontinuity: bool,
}
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

/// Scans a library, or every one. It waits for a scan already running, and libraries asked for
/// meanwhile are scanned together after it.
pub struct ScanRequested {
    /// `None` scans every library.
    pub library: Option<Locator>,
    /// `false` reloads every track's metadata instead of trusting the cache.
    pub check_cache: bool,
}
/// The library's tracks, after a scan listed its libraries or a library was removed; those not
/// loaded yet are placeholders. At launch, the scan first shows what the libraries listed when
/// last listed.
pub struct TracksEnumerated {
    pub tracks: Vec<Track>,
}
/// A track's metadata, replacing its placeholder in [`TracksEnumerated`].
pub struct TrackMetadataLoaded {
    pub track: Track,
}
/// Where scanning is, sent when a scan is asked for, lists its libraries or finishes. Tracks read
/// since come as [`TrackMetadataLoaded`].
pub struct ScanProgress {
    /// The libraries being scanned or waiting for a scan; empty when none is.
    pub libraries: Vec<Locator>,
    /// The tracks the running scan listed, and how many of those it still reads.
    pub found: usize,
    pub pending: usize,
}
pub struct ScanFinished {
    pub libraries: Vec<Locator>,
    /// The scan ran to the end over each of `libraries`.
    pub complete: bool,
}

/// Replaces the library folders: scans the ones added and takes the tracks of the ones removed
/// out of the library.
pub struct SetLibraryRoots(pub Vec<Locator>);
/// The library folders, at startup and after every change.
pub struct LibraryRootsChanged(pub Vec<Locator>);
/// Calls a library `name`, or (`None`) by its folder's or playlist's name again.
pub struct RenameLibrary {
    pub library: Locator,
    pub name: Option<String>,
}
/// A library was renamed; names are in the [`crate::library::catalog::Library`].
pub struct LibraryRenamed {
    pub library: Locator,
    pub name: Option<String>,
}

/// Keeps copies of streamed tracks on disk as they play, in at most `limit` bytes: when space
/// runs out, the most played tracks keep theirs. Turning it off deletes the copies.
pub struct SetStreamCache {
    pub enabled: bool,
    pub limit: u64,
}
/// The stream cache's settings and the bytes its copies take, at startup and after every
/// change.
pub struct StreamCacheChanged {
    pub enabled: bool,
    pub limit: u64,
    pub used: u64,
}

/// Whether the app is in front; positions are not reported while it is not.
pub struct AppVisibilityChanged(pub bool);

messages!(
    SetStreamCache,
    StreamCacheChanged,
    AppVisibilityChanged,
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
    SetOutputDevice,
    ListOutputDevices,
    OutputDevices,
    Seek,
    SetVolume,
    SetLoopStatus,
    ToggleRepeat,
    SetShuffle,
    ToggleShuffle,
    SetReplayGain,
    SetResume,
    SetCrossfade,
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
    ScanRequested,
    TracksEnumerated,
    TrackMetadataLoaded,
    ScanProgress,
    ScanFinished,
    SetLibraryRoots,
    LibraryRootsChanged,
    RenameLibrary,
    LibraryRenamed,
);
