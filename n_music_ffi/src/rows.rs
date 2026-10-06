//! Flat copies of what the screens show, so Kotlin never holds the core's shared tracks.

use n_music_core::library::catalog::Catalog;
use n_music_core::queue::{ItemId, QueueEntry};
use n_music_core::source::Locator;
use n_music_core::TrackInfo;

/// A track as lists and the player show it.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct TrackRow {
    pub locator: Locator,
    /// Never empty: falls back to the file name.
    pub title: String,
    /// The artists on one line; empty when there is none.
    pub artist: String,
    pub album: Option<String>,
    /// In seconds; 0 while unknown.
    pub length: f64,
    /// Path of the cover thumbnail, when the track has one.
    pub cover: Option<String>,
}

impl From<&TrackInfo> for TrackRow {
    fn from(track: &TrackInfo) -> Self {
        Self {
            locator: track.locator.clone(),
            title: track.title.clone(),
            artist: track.artist(),
            album: track.album.clone(),
            length: track.length,
            cover: track
                .cover
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
        }
    }
}

/// An entry of the play session, in play order.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct QueueRow {
    /// Tells the entry apart even when its track is in the session twice.
    pub item: ItemId,
    /// Added with `Enqueue` rather than taken from what plays.
    pub queued: bool,
    pub track: TrackRow,
}

impl QueueRow {
    /// The entry with what the library knows of its track, or its file name until then.
    pub fn new(entry: &QueueEntry, catalog: &Catalog) -> Self {
        let track = match catalog.track(&entry.locator) {
            Some(track) => TrackRow::from(&**track),
            None => TrackRow::from(&TrackInfo::placeholder(entry.locator.clone())),
        };
        Self {
            item: entry.item,
            queued: entry.queued,
            track,
        }
    }
}
