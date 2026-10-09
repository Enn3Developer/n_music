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
    /// Each artist, the first one being the track's artist page.
    pub artists: Vec<String>,
    pub album: Option<String>,
    /// The artist the album is listed under: its album artist, else the track's first artist.
    pub album_artist: Option<String>,
    pub year: Option<i32>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    /// In seconds; 0 while unknown.
    pub length: f64,
    /// Path of the cover thumbnail, when the track has one.
    pub cover: Option<String>,
    /// How many times it was listened to past its middle.
    pub plays: u32,
    /// Its tags were read: a scan lists tracks first and reads them after.
    pub loaded: bool,
}

impl TrackRow {
    /// `track` with its plays from `catalog`.
    pub fn new(track: &TrackInfo, catalog: &Catalog) -> Self {
        let mut row = Self::from(track);
        row.plays = catalog.stats(&track.locator).map_or(0, |stats| stats.plays);
        row
    }
}

impl From<&TrackInfo> for TrackRow {
    /// `track` without its plays, which the catalog keeps.
    fn from(track: &TrackInfo) -> Self {
        Self {
            locator: track.locator.clone(),
            title: track.title.clone(),
            artist: track.artist(),
            artists: track.artists.clone(),
            album: track.album.clone(),
            album_artist: track
                .album_artist
                .clone()
                .or_else(|| track.artists.first().cloned()),
            year: track.year,
            track_number: track.track_number,
            disc_number: track.disc_number,
            length: track.length,
            cover: track
                .cover
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            plays: 0,
            loaded: track.length > 0.0 || track.codec.is_some(),
        }
    }
}

/// What a track's menu and the player add to its row.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct TrackDetails {
    /// Short codec name, e.g. `flac`.
    pub codec: Option<String>,
    /// In Hz.
    pub sample_rate: Option<u32>,
    pub bits_per_sample: Option<u32>,
    pub channels: Option<u32>,
    pub genres: Vec<String>,
    pub plays: u32,
    /// Unix seconds; `None` when it was never played.
    pub last_played: Option<i64>,
}

impl TrackDetails {
    pub fn new(track: &TrackInfo, catalog: &Catalog) -> Self {
        let stats = catalog.stats(&track.locator);
        Self {
            codec: track.codec.clone(),
            sample_rate: track.sample_rate,
            bits_per_sample: track.bits_per_sample,
            channels: track.channels,
            genres: track.genres.clone(),
            plays: stats.map_or(0, |stats| stats.plays),
            last_played: stats.map(|stats| stats.last_played),
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
            Some(track) => TrackRow::new(track, catalog),
            None => TrackRow::from(&TrackInfo::placeholder(entry.locator.clone())),
        };
        Self {
            item: entry.item,
            queued: entry.queued,
            track,
        }
    }
}
