//! The core's own types, exported as they are. UniFFI needs each definition restated here; a
//! restatement that drifts from the core's no longer compiles.

use n_music_core::library::query::{Filter, PlaylistId, Query, SortField, SortKey, Tag};
use n_music_core::library::track::ReplayGainMode;
use n_music_core::messages::{PlaylistSummary, Seek};
use n_music_core::queue::{ItemId, LoopStatus};
use n_music_core::settings::OutputDevice;
use n_music_core::source::telegram::{
    TelegramChatInfo, TelegramChatKind, TelegramError, TelegramStatus,
};
use n_music_core::source::Locator;

uniffi::custom_type!(ItemId, u64, {
    remote,
    lower: |item| item.0,
    try_lift: |value| Ok(ItemId(value)),
});

uniffi::custom_type!(PlaylistId, i64, {
    remote,
    lower: |id| id.0,
    try_lift: |value| Ok(PlaylistId(value)),
});

#[uniffi::remote(Enum)]
pub enum Locator {
    Local(String),
    DocumentTree(String),
    Document { uri: String, name: String },
    Web(String),
    TelegramChat(String),
    TelegramAudio { uri: String, name: String },
}

#[uniffi::remote(Enum)]
pub enum TelegramStatus {
    SignedOut,
    CodeSent { phone: String },
    PasswordNeeded { hint: Option<String> },
    SignedIn { name: String },
}

#[uniffi::remote(Enum)]
pub enum TelegramError {
    PhoneInvalid,
    PhoneBanned,
    CodeInvalid,
    PasswordInvalid,
    SignUpRequired,
    Wait { seconds: u32 },
    SignedOut,
    NotFound,
    Offline(String),
    Failed(String),
}

#[uniffi::remote(Record)]
pub struct TelegramChatInfo {
    pub locator: Locator,
    pub title: String,
    pub kind: TelegramChatKind,
    pub username: Option<String>,
}

#[uniffi::remote(Enum)]
pub enum TelegramChatKind {
    SavedMessages,
    User,
    Bot,
    Group,
    Channel,
}

#[uniffi::remote(Enum)]
pub enum LoopStatus {
    Off,
    Playlist,
    File,
}

#[uniffi::remote(Enum)]
pub enum ReplayGainMode {
    Off,
    Track,
    Album,
}

#[uniffi::remote(Record)]
pub struct OutputDevice {
    pub id: String,
    pub name: String,
}

#[uniffi::remote(Record)]
pub struct PlaylistSummary {
    pub id: PlaylistId,
    pub name: String,
    pub smart: bool,
}

#[uniffi::remote(Enum)]
pub enum Seek {
    Absolute(f64),
    Relative(f64),
    Tracked { position: f64, request: u64 },
    ToItem { item: ItemId, position: f64 },
}

#[uniffi::remote(Record)]
pub struct Query {
    pub filter: Filter,
    pub sort: Vec<SortKey>,
}

#[uniffi::remote(Enum)]
pub enum Filter {
    All(Vec<Filter>),
    Any(Vec<Filter>),
    Not(Box<Filter>),
    Playlist(PlaylistId),
    Artist(String),
    AlbumArtist(String),
    Album(String),
    Genre(String),
    Codec(String),
    Untagged(Tag),
    Year { from: Option<i32>, to: Option<i32> },
    Search(String),
    Folder(String),
    Library(Locator),
    Plays { min: Option<u32>, max: Option<u32> },
    PlayedWithin { seconds: u64 },
    NotPlayedWithin { seconds: u64 },
}

#[uniffi::remote(Enum)]
pub enum Tag {
    Artist,
    Album,
    Genre,
}

#[uniffi::remote(Record)]
pub struct SortKey {
    pub field: SortField,
    pub descending: bool,
}

#[uniffi::remote(Enum)]
pub enum SortField {
    Title,
    Artist,
    Album,
    Genre,
    Year,
    Length,
    Codec,
    Plays,
    LastPlayed,
    Added(PlaylistId),
    Location,
}

/// The whole library, in the order the scan lists it: what the interim screens show and play.
#[uniffi::export]
pub fn library_query() -> Query {
    Query::library()
}
