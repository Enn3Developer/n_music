//! Which tracks to play and in what order: a filter over the library plus a sort. Playlists are
//! sets of tracks, or saved filters (smart playlists); either way the order comes from a sort.

use super::catalog::Catalog;
use crate::source::Locator;
use crate::{Track, TrackInfo};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PlaylistId(pub i64);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub filter: Filter,
    /// Ties are broken by location, so the order is always the same.
    pub sort: Vec<SortKey>,
}

impl Query {
    /// The whole library, in the order the scan lists it.
    pub fn library() -> Self {
        Self {
            filter: Filter::All(vec![]),
            sort: vec![SortKey::ascending(SortField::Location)],
        }
    }

    /// A playlist's tracks in its own sort.
    pub fn playlist(catalog: &Catalog, id: PlaylistId) -> Self {
        Self {
            filter: Filter::Playlist(id),
            sort: catalog
                .playlist(id)
                .map(|playlist| playlist.sort.clone())
                .unwrap_or_default(),
        }
    }
}

/// Text comparisons ignore case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Filter {
    /// Every filter matches; `All(vec![])` matches everything.
    All(Vec<Filter>),
    /// At least one filter matches.
    Any(Vec<Filter>),
    Not(Box<Filter>),
    /// In a playlist, or matching a smart playlist's rule.
    Playlist(PlaylistId),
    /// One of the track's artists.
    Artist(String),
    AlbumArtist(String),
    Album(String),
    /// One of the track's genres.
    Genre(String),
    /// Short codec name, e.g. `flac`.
    Codec(String),
    /// The track has no value for this tag.
    Untagged(Tag),
    /// Released within these years, both inclusive.
    Year {
        from: Option<i32>,
        to: Option<i32>,
    },
    /// The title, an artist or the album contains the text.
    Search(String),
    /// Somewhere under this folder (a location prefix).
    Folder(String),
    /// Listed by this library when it was last scanned.
    Library(Locator),
    /// Played this many times, both bounds inclusive.
    Plays {
        min: Option<u32>,
        max: Option<u32>,
    },
    /// Played within the last `seconds`.
    PlayedWithin {
        seconds: u64,
    },
    /// Not played within the last `seconds`, or never.
    NotPlayedWithin {
        seconds: u64,
    },
}

/// A tag a track can lack, see [`Filter::Untagged`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tag {
    Artist,
    Album,
    Genre,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortKey {
    pub field: SortField,
    pub descending: bool,
}

impl SortKey {
    pub fn ascending(field: SortField) -> Self {
        Self {
            field,
            descending: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SortField {
    Title,
    /// The first artist.
    Artist,
    /// Album artist, album, disc and track number.
    Album,
    /// The genres, in the order the track lists them.
    Genre,
    Year,
    Length,
    /// Short codec name, e.g. `flac`.
    Codec,
    Plays,
    LastPlayed,
    /// When the track was added to this playlist; tracks not in it sort last.
    Added(PlaylistId),
    /// The location as the scan lists it.
    Location,
}

/// A sort value; missing values sort after every other.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Number(i64),
    Text(String),
    Missing,
}

impl Key {
    fn text(value: Option<&str>) -> Self {
        value.map_or(Key::Missing, |value| Key::Text(value.to_lowercase()))
    }

    fn number(value: Option<i64>) -> Self {
        value.map_or(Key::Missing, Key::Number)
    }
}

pub(crate) fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64)
}

impl Catalog {
    /// The tracks `query` selects, sorted.
    pub fn select(&self, query: &Query) -> Vec<Track> {
        let now = now();
        let mut keyed: Vec<(Vec<Key>, String, Track)> = self
            .tracks()
            .iter()
            .filter(|track| self.matches(&query.filter, track, now, &mut vec![]))
            .map(|track| {
                let keys = query
                    .sort
                    .iter()
                    .map(|key| self.key(&key.field, track))
                    .collect();
                (keys, track.locator.to_string(), track.clone())
            })
            .collect();
        keyed.sort_by(|(a, a_location, _), (b, b_location, _)| {
            a.iter()
                .zip(b)
                .zip(&query.sort)
                .map(|((a, b), key)| {
                    // Missing values stay last in both directions.
                    match (a, b) {
                        (Key::Missing, Key::Missing) => Ordering::Equal,
                        (Key::Missing, _) => Ordering::Greater,
                        (_, Key::Missing) => Ordering::Less,
                        _ if key.descending => b.cmp(a),
                        _ => a.cmp(b),
                    }
                })
                .find(|ordering| ordering.is_ne())
                .unwrap_or_else(|| a_location.cmp(b_location))
        });
        keyed.into_iter().map(|(_, _, track)| track).collect()
    }

    /// Whether `filter` holds for `track`. `visiting` holds the smart playlists being
    /// evaluated: a rule that refers back to itself matches nothing.
    fn matches(
        &self,
        filter: &Filter,
        track: &TrackInfo,
        now: i64,
        visiting: &mut Vec<PlaylistId>,
    ) -> bool {
        let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
        match filter {
            Filter::All(filters) => filters
                .iter()
                .all(|filter| self.matches(filter, track, now, visiting)),
            Filter::Any(filters) => filters
                .iter()
                .any(|filter| self.matches(filter, track, now, visiting)),
            Filter::Not(filter) => !self.matches(filter, track, now, visiting),
            Filter::Playlist(id) => {
                let Some(playlist) = self.playlist(*id) else {
                    return false;
                };
                match &playlist.rule {
                    None => playlist.items.contains_key(&track.locator),
                    Some(_) if visiting.contains(id) => false,
                    Some(rule) => {
                        visiting.push(*id);
                        let matches = self.matches(rule, track, now, visiting);
                        visiting.pop();
                        matches
                    }
                }
            }
            Filter::Artist(artist) => track.artists.iter().any(|value| same(value, artist)),
            Filter::AlbumArtist(artist) => track
                .album_artist
                .as_deref()
                .is_some_and(|value| same(value, artist)),
            Filter::Album(album) => track
                .album
                .as_deref()
                .is_some_and(|value| same(value, album)),
            Filter::Genre(genre) => track.genres.iter().any(|value| same(value, genre)),
            Filter::Codec(codec) => track
                .codec
                .as_deref()
                .is_some_and(|value| same(value, codec)),
            Filter::Untagged(Tag::Artist) => track.artists.is_empty(),
            Filter::Untagged(Tag::Album) => track.album.is_none(),
            Filter::Untagged(Tag::Genre) => track.genres.is_empty(),
            Filter::Year { from, to } => track.year.is_some_and(|year| {
                from.is_none_or(|from| year >= from) && to.is_none_or(|to| year <= to)
            }),
            Filter::Search(text) => {
                let text = text.to_lowercase();
                let contains = |value: &str| value.to_lowercase().contains(&text);
                contains(&track.title)
                    || track.artists.iter().any(|artist| contains(artist))
                    || track.album.as_deref().is_some_and(contains)
            }
            Filter::Folder(folder) => location(&track.locator).starts_with(folder.as_str()),
            Filter::Library(library) => self
                .listed(library)
                .is_some_and(|listed| listed.tracks.contains(&track.locator)),
            Filter::Plays { min, max } => {
                let plays = self.stats(&track.locator).map_or(0, |stats| stats.plays);
                min.is_none_or(|min| plays >= min) && max.is_none_or(|max| plays <= max)
            }
            Filter::PlayedWithin { seconds } => self
                .stats(&track.locator)
                .is_some_and(|stats| now - stats.last_played <= *seconds as i64),
            Filter::NotPlayedWithin { seconds } => self
                .stats(&track.locator)
                .is_none_or(|stats| now - stats.last_played > *seconds as i64),
        }
    }

    fn key(&self, field: &SortField, track: &TrackInfo) -> Key {
        match field {
            SortField::Title => Key::text(Some(&track.title)),
            SortField::Artist => Key::text(track.artists.first().map(String::as_str)),
            SortField::Album => Key::Text(format!(
                "{}\u{0}{}\u{0}{:05}\u{0}{:05}",
                track
                    .album_artist
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase(),
                track.album.as_deref().unwrap_or_default().to_lowercase(),
                track.disc_number.unwrap_or(0),
                track.track_number.unwrap_or(0),
            )),
            SortField::Genre if track.genres.is_empty() => Key::Missing,
            SortField::Genre => Key::text(Some(&track.genres.join(", "))),
            SortField::Year => Key::number(track.year.map(i64::from)),
            SortField::Length => Key::Number((track.length * 1000.0) as i64),
            SortField::Codec => Key::text(track.codec.as_deref()),
            SortField::Plays => Key::Number(
                self.stats(&track.locator)
                    .map_or(0, |stats| i64::from(stats.plays)),
            ),
            SortField::LastPlayed => {
                Key::number(self.stats(&track.locator).map(|stats| stats.last_played))
            }
            SortField::Added(id) => Key::number(
                self.playlist(*id)
                    .and_then(|playlist| playlist.items.get(&track.locator))
                    .map(|item| item.added),
            ),
            // Not lowercased: the scan sorts by the exact text.
            SortField::Location => Key::Text(track.locator.to_string()),
        }
    }
}

/// The text a [`Filter::Folder`] prefix is matched against.
fn location(locator: &Locator) -> &str {
    match locator {
        Locator::Local(path) | Locator::DocumentTree(path) | Locator::Web(path) => path,
        Locator::Document { uri, .. } => uri,
    }
}

impl Filter {
    /// Whether this filter refers to smart playlist `id`, directly or through other smart
    /// playlists: saving such a rule in `id` would make it depend on itself.
    pub fn refers_to(&self, catalog: &Catalog, id: PlaylistId) -> bool {
        self.refers(catalog, id, &mut vec![])
    }

    fn refers(&self, catalog: &Catalog, id: PlaylistId, seen: &mut Vec<PlaylistId>) -> bool {
        match self {
            Filter::All(filters) | Filter::Any(filters) => filters
                .iter()
                .any(|filter| filter.refers(catalog, id, seen)),
            Filter::Not(filter) => filter.refers(catalog, id, seen),
            Filter::Playlist(other) if *other == id => true,
            Filter::Playlist(other) if seen.contains(other) => false,
            Filter::Playlist(other) => {
                seen.push(*other);
                catalog
                    .playlist(*other)
                    .and_then(|playlist| playlist.rule.as_ref())
                    .is_some_and(|rule| rule.refers(catalog, id, seen))
            }
            _ => false,
        }
    }
}
