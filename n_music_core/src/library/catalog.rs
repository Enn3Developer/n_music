//! The library in memory: the scanned tracks, the playlists and the play statistics. Queries
//! run here rather than in SQL, because the database lags behind a running scan and does not
//! keep tracks the provider cannot version.

use super::query::{Filter, PlaylistId, SortKey};
use crate::source::Locator;
use crate::Track;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Clone, Debug, PartialEq)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: String,
    /// A smart playlist's rule; `None` for one of explicitly added tracks.
    pub rule: Option<Filter>,
    pub sort: Vec<SortKey>,
    /// The added tracks; a smart playlist keeps any it had before getting its rule.
    pub items: HashMap<Locator, PlaylistItem>,
    /// Unix seconds.
    pub created: i64,
    pub modified: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaylistItem {
    /// Unix seconds.
    pub added: i64,
    pub fingerprint: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayStats {
    pub plays: u32,
    /// Unix seconds.
    pub last_played: i64,
}

#[derive(Default)]
pub struct Catalog {
    /// In scan order.
    tracks: Vec<Track>,
    by_locator: HashMap<Locator, usize>,
    by_fingerprint: HashMap<u64, usize>,
    playlists: BTreeMap<PlaylistId, Playlist>,
    stats: HashMap<Locator, PlayStats>,
}

impl Catalog {
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn track(&self, locator: &Locator) -> Option<&Track> {
        self.by_locator
            .get(locator)
            .map(|&index| &self.tracks[index])
    }

    /// A track with this fingerprint, wherever it is now.
    pub fn track_by_fingerprint(&self, fingerprint: u64) -> Option<&Track> {
        self.by_fingerprint
            .get(&fingerprint)
            .map(|&index| &self.tracks[index])
    }

    pub fn playlist(&self, id: PlaylistId) -> Option<&Playlist> {
        self.playlists.get(&id)
    }

    pub fn playlists(&self) -> impl Iterator<Item = &Playlist> {
        self.playlists.values()
    }

    pub fn stats(&self, locator: &Locator) -> Option<&PlayStats> {
        self.stats.get(locator)
    }

    pub(crate) fn set_tracks(&mut self, tracks: Vec<Track>) {
        self.by_locator = tracks
            .iter()
            .enumerate()
            .map(|(index, track)| (track.locator.clone(), index))
            .collect();
        self.by_fingerprint = tracks
            .iter()
            .enumerate()
            .filter_map(|(index, track)| Some((track.fingerprint?, index)))
            .collect();
        self.tracks = tracks;
    }

    /// Replaces the track at scan `index` with its freshly read metadata.
    pub(crate) fn update_track(&mut self, index: usize, track: Track) {
        let Some(slot) = self.tracks.get_mut(index) else {
            return;
        };
        if slot.locator != track.locator {
            return;
        }
        if let Some(fingerprint) = track.fingerprint {
            self.by_fingerprint.insert(fingerprint, index);
        }
        *slot = track;
    }

    pub(crate) fn set_playlists(&mut self, playlists: Vec<Playlist>) {
        self.playlists = playlists
            .into_iter()
            .map(|playlist| (playlist.id, playlist))
            .collect();
    }

    pub(crate) fn playlist_mut(&mut self, id: PlaylistId) -> Option<&mut Playlist> {
        self.playlists.get_mut(&id)
    }

    pub(crate) fn insert_playlist(&mut self, playlist: Playlist) {
        self.playlists.insert(playlist.id, playlist);
    }

    pub(crate) fn remove_playlist(&mut self, id: PlaylistId) {
        self.playlists.remove(&id);
    }

    pub(crate) fn set_stats(&mut self, stats: HashMap<Locator, PlayStats>) {
        self.stats = stats;
    }

    pub(crate) fn record_play(&mut self, locator: Locator, now: i64) {
        let stats = self.stats.entry(locator).or_default();
        stats.plays += 1;
        stats.last_played = now;
    }
}

/// Shared handle to the [`Catalog`]: the library service writes it, the queue reads it.
#[derive(Clone, Default)]
pub struct Library(Arc<RwLock<Catalog>>);

impl Library {
    pub fn read(&self) -> RwLockReadGuard<'_, Catalog> {
        self.0.read().unwrap()
    }

    pub(crate) fn write(&self) -> RwLockWriteGuard<'_, Catalog> {
        self.0.write().unwrap()
    }
}
