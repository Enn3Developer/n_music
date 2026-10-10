//! What Kotlin asks of the core: one variant per command message, sent by [`Command::emit`].

use n_event_bus::EventWriter;
use n_music_core::library::query::{Filter, PlaylistId, Query, SortKey};
use n_music_core::library::track::ReplayGainMode;
use n_music_core::messages::{self, Seek};
use n_music_core::queue::{ItemId, LoopStatus};
use n_music_core::settings::OutputDevice;
use n_music_core::source::Locator;

/// A command for the core. Each variant is the message of the same name in
/// `n_music_core::messages`, which documents it.
#[derive(uniffi::Enum)]
pub enum Command {
    PlayFrom {
        query: Query,
        start: Option<Locator>,
    },
    Enqueue {
        tracks: Vec<Locator>,
        next: bool,
    },
    RemoveQueued {
        item: ItemId,
    },
    ClearQueued,
    MoveUpcoming {
        item: ItemId,
        before: Option<ItemId>,
    },
    MoveCurrent {
        item: ItemId,
        before: Option<ItemId>,
    },
    PlayPrevious,
    PlayNext,
    TogglePause,
    Pause,
    Play,
    /// The system's audio devices changed, e.g. headphones were plugged in.
    OutputDeviceChanged,
    SetOutputDevice {
        device: Option<OutputDevice>,
    },
    /// Answered with `CoreEvent.OutputDevices`.
    ListOutputDevices,
    Seek {
        seek: Seek,
    },
    SetVolume {
        volume: f64,
    },
    SetLoopStatus {
        status: LoopStatus,
    },
    ToggleRepeat,
    SetShuffle {
        enabled: bool,
    },
    ToggleShuffle,
    SetReplayGain {
        mode: ReplayGainMode,
    },
    SetResume {
        enabled: bool,
    },
    SetCrossfade {
        seconds: f64,
    },
    CreatePlaylist {
        name: String,
        rule: Option<Filter>,
        sort: Vec<SortKey>,
        tracks: Vec<Locator>,
    },
    RenamePlaylist {
        id: PlaylistId,
        name: String,
    },
    DeletePlaylist {
        id: PlaylistId,
    },
    SetPlaylistRule {
        id: PlaylistId,
        rule: Option<Filter>,
    },
    SetPlaylistSort {
        id: PlaylistId,
        sort: Vec<SortKey>,
    },
    AddToPlaylist {
        id: PlaylistId,
        tracks: Vec<Locator>,
    },
    RemoveFromPlaylist {
        id: PlaylistId,
        tracks: Vec<Locator>,
    },
    ScanRequested {
        library: Option<Locator>,
        check_cache: bool,
    },
    SetLibraryRoots {
        roots: Vec<Locator>,
    },
    RenameLibrary {
        library: Locator,
        name: Option<String>,
    },
    SetStreamCache {
        enabled: bool,
        limit: u64,
    },
    /// Whether the app is in front. Positions come four times a second while it is, and only
    /// as often as the play session is saved while it is not.
    AppVisibilityChanged {
        visible: bool,
    },
    /// Each Telegram step is answered with `CoreEvent.TelegramStatusChanged` as it starts and
    /// ends. One sent while another runs is dropped.
    TelegramSignIn {
        phone: String,
    },
    TelegramCode {
        code: String,
    },
    TelegramPassword {
        password: String,
    },
    TelegramSignOut,
    /// Answered with `CoreEvent.TelegramChatsFound`; a newer one replaces it.
    FindTelegramChats {
        query: String,
    },
}

impl Command {
    /// Sends the message this command stands for. It never blocks.
    pub fn emit(self, writer: &EventWriter) {
        match self {
            Command::PlayFrom { query, start } => writer.emit(messages::PlayFrom { query, start }),
            Command::Enqueue { tracks, next } => writer.emit(messages::Enqueue { tracks, next }),
            Command::RemoveQueued { item } => writer.emit(messages::RemoveQueued(item)),
            Command::ClearQueued => writer.emit(messages::ClearQueued),
            Command::MoveUpcoming { item, before } => {
                writer.emit(messages::MoveUpcoming { item, before })
            }
            Command::MoveCurrent { item, before } => {
                writer.emit(messages::MoveCurrent { item, before })
            }
            Command::PlayPrevious => writer.emit(messages::PlayPrevious),
            Command::PlayNext => writer.emit(messages::PlayNext),
            Command::TogglePause => writer.emit(messages::TogglePause),
            Command::Pause => writer.emit(messages::Pause),
            Command::Play => writer.emit(messages::Play),
            Command::OutputDeviceChanged => writer.emit(messages::OutputDeviceChanged),
            Command::SetOutputDevice { device } => writer.emit(messages::SetOutputDevice(device)),
            Command::ListOutputDevices => writer.emit(messages::ListOutputDevices),
            Command::Seek { seek } => writer.emit(seek),
            Command::SetVolume { volume } => writer.emit(messages::SetVolume(volume)),
            Command::SetLoopStatus { status } => writer.emit(messages::SetLoopStatus(status)),
            Command::ToggleRepeat => writer.emit(messages::ToggleRepeat),
            Command::SetShuffle { enabled } => writer.emit(messages::SetShuffle(enabled)),
            Command::ToggleShuffle => writer.emit(messages::ToggleShuffle),
            Command::SetReplayGain { mode } => writer.emit(messages::SetReplayGain(mode)),
            Command::SetResume { enabled } => writer.emit(messages::SetResume(enabled)),
            Command::SetCrossfade { seconds } => writer.emit(messages::SetCrossfade(seconds)),
            Command::CreatePlaylist {
                name,
                rule,
                sort,
                tracks,
            } => writer.emit(messages::CreatePlaylist {
                name,
                rule,
                sort,
                tracks,
            }),
            Command::RenamePlaylist { id, name } => {
                writer.emit(messages::RenamePlaylist { id, name })
            }
            Command::DeletePlaylist { id } => writer.emit(messages::DeletePlaylist(id)),
            Command::SetPlaylistRule { id, rule } => {
                writer.emit(messages::SetPlaylistRule { id, rule })
            }
            Command::SetPlaylistSort { id, sort } => {
                writer.emit(messages::SetPlaylistSort { id, sort })
            }
            Command::AddToPlaylist { id, tracks } => {
                writer.emit(messages::AddToPlaylist { id, tracks })
            }
            Command::RemoveFromPlaylist { id, tracks } => {
                writer.emit(messages::RemoveFromPlaylist { id, tracks })
            }
            Command::ScanRequested {
                library,
                check_cache,
            } => writer.emit(messages::ScanRequested {
                library,
                check_cache,
            }),
            Command::SetLibraryRoots { roots } => writer.emit(messages::SetLibraryRoots(roots)),
            Command::RenameLibrary { library, name } => {
                writer.emit(messages::RenameLibrary { library, name })
            }
            Command::SetStreamCache { enabled, limit } => {
                writer.emit(messages::SetStreamCache { enabled, limit })
            }
            Command::AppVisibilityChanged { visible } => {
                writer.emit(messages::AppVisibilityChanged(visible))
            }
            Command::TelegramSignIn { phone } => writer.emit(messages::TelegramSignIn { phone }),
            Command::TelegramCode { code } => writer.emit(messages::TelegramCode(code)),
            Command::TelegramPassword { password } => {
                writer.emit(messages::TelegramPassword(password))
            }
            Command::TelegramSignOut => writer.emit(messages::TelegramSignOut),
            Command::FindTelegramChats { query } => {
                writer.emit(messages::FindTelegramChats { query })
            }
        }
    }
}
