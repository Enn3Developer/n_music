//! The library as the screens list it besides tracks: albums, artists and genres, the
//! playlists, the sources, and the values a filter can pick from.

use n_music_core::library::catalog::Catalog;
use n_music_core::library::query::{Filter, PlaylistId, Query, SortKey};
use n_music_core::source::Locator;
use n_music_core::Track;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// How many covers a playlist or a genre shows in its 2 × 2 mosaic.
const MOSAIC: usize = 4;

/// The order of albums, artists or genres.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum GroupSort {
    /// By name.
    Name,
    /// By the album's artist, then its name.
    Artist,
    /// The latest year first.
    Newest,
    /// The earliest year first.
    Oldest,
    /// The most tracks first.
    MostTracks,
}

/// An album: tracks sharing an album name and its artist. `name` is `None` for the tracks
/// without an album, which all share one.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AlbumRow {
    pub name: Option<String>,
    pub artist: Option<String>,
    pub year: Option<i32>,
    pub tracks: u32,
    /// In seconds.
    pub length: f64,
    pub cover: Option<String>,
}

/// An artist, or (`name` `None`) the tracks without one.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ArtistRow {
    pub name: Option<String>,
    pub tracks: u32,
    pub length: f64,
    pub cover: Option<String>,
}

/// A genre, or (`name` `None`) the tracks without one.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct GenreRow {
    pub name: Option<String>,
    pub tracks: u32,
    /// Up to four covers of different albums.
    pub covers: Vec<String>,
}

/// How many tracks a selection holds and how long they play.
#[derive(Clone, Copy, Debug, PartialEq, uniffi::Record)]
pub struct Summary {
    pub tracks: u32,
    /// In seconds.
    pub length: f64,
}

/// A playlist as its row and its page show it.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PlaylistRow {
    pub id: PlaylistId,
    pub name: String,
    /// A smart playlist's rule; `None` for one of added tracks.
    pub rule: Option<Filter>,
    pub sort: Vec<SortKey>,
    /// Its tracks the library has.
    pub tracks: u32,
    pub length: f64,
    /// Up to four covers of different albums, in its order.
    pub covers: Vec<String>,
    /// Unix seconds.
    pub created: i64,
    pub modified: i64,
}

/// A library root as Sources lists it.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SourceRow {
    pub root: Locator,
    /// The name it was given; `None` when it goes by its folder's or playlist's.
    pub name: Option<String>,
    /// The tracks of the library it listed.
    pub tracks: u32,
    pub length: f64,
    pub cover: Option<String>,
    /// A scan listed it, in this launch or an earlier one.
    pub listed: bool,
    /// The last scan could reach it. One that could not keeps only the tracks saved for offline.
    pub reachable: bool,
}

/// A value a filter can pick and how many tracks have it.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct Facet {
    pub name: String,
    pub tracks: u32,
}

/// What the library's tracks have, for the filters to offer.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct Facets {
    /// The most common first.
    pub genres: Vec<Facet>,
    /// Short codec names, the most common first.
    pub codecs: Vec<Facet>,
    pub first_year: Option<i32>,
    pub last_year: Option<i32>,
}

/// The tracks `filter` selects, in scan order: the whole library without a filter.
pub fn selected(catalog: &Catalog, filter: &Filter) -> Vec<Track> {
    match filter {
        Filter::All(filters) if filters.is_empty() => catalog.tracks().to_vec(),
        _ => catalog.select(&Query {
            filter: filter.clone(),
            sort: vec![],
        }),
    }
}

pub fn summary(tracks: &[Track]) -> Summary {
    Summary {
        tracks: count(tracks.len()),
        length: tracks.iter().map(|track| track.length).sum(),
    }
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn cover(track: &Track) -> Option<String> {
    track
        .cover
        .as_deref()
        .map(|path| path.to_string_lossy().into_owned())
}

/// Text compared without case.
fn key(text: Option<&str>) -> String {
    text.unwrap_or_default().to_lowercase()
}

fn contains(text: Option<&str>, search: &str) -> bool {
    text.is_some_and(|text| text.to_lowercase().contains(search))
}

/// Up to [`MOSAIC`] covers of `tracks`, each from a different album.
fn mosaic<'a>(tracks: impl IntoIterator<Item = &'a Track>) -> Vec<String> {
    let mut albums = HashSet::new();
    let mut covers = vec![];
    for track in tracks {
        let Some(path) = cover(track) else {
            continue;
        };
        if albums.insert(key(track.album.as_deref())) && !covers.contains(&path) {
            covers.push(path);
            if covers.len() == MOSAIC {
                break;
            }
        }
    }
    covers
}

/// The albums of `tracks` whose name or artist contains `search`, in the order of `sort`.
pub fn albums(tracks: &[Track], search: &str, sort: GroupSort) -> Vec<AlbumRow> {
    let mut rows: Vec<AlbumRow> = vec![];
    let mut index: HashMap<(String, String), usize> = HashMap::new();
    for track in tracks {
        let (name, artist) = match track.album.as_deref() {
            Some(album) => (
                Some(album),
                track
                    .album_artist
                    .as_deref()
                    .or(track.artists.first().map(String::as_str)),
            ),
            None => (None, None),
        };
        let position = *index.entry((key(name), key(artist))).or_insert_with(|| {
            rows.push(AlbumRow {
                name: name.map(String::from),
                artist: artist.map(String::from),
                year: None,
                tracks: 0,
                length: 0.0,
                cover: None,
            });
            rows.len() - 1
        });
        let row = &mut rows[position];
        row.tracks += 1;
        row.length += track.length;
        row.year = row.year.or(track.year);
        if row.cover.is_none() {
            row.cover = cover(track);
        }
    }
    let search = search.trim().to_lowercase();
    if !search.is_empty() {
        rows.retain(|row| {
            contains(row.name.as_deref(), &search) || contains(row.artist.as_deref(), &search)
        });
    }
    match sort {
        GroupSort::Artist => {
            rows.sort_by_cached_key(|row| (key(row.artist.as_deref()), key(row.name.as_deref())))
        }
        GroupSort::Newest => rows.sort_by_cached_key(|row| {
            (
                row.year.is_none(),
                Reverse(row.year),
                key(row.name.as_deref()),
            )
        }),
        GroupSort::Oldest => {
            rows.sort_by_cached_key(|row| (row.year.is_none(), row.year, key(row.name.as_deref())))
        }
        GroupSort::MostTracks => {
            rows.sort_by_cached_key(|row| (Reverse(row.tracks), key(row.name.as_deref())))
        }
        GroupSort::Name => rows.sort_by_cached_key(|row| key(row.name.as_deref())),
    }
    // The tracks without an album come last, whatever the order.
    rows.sort_by_key(|row| row.name.is_none());
    rows
}

/// The artists of `tracks` whose name contains `search`; a track with several artists counts
/// for each.
pub fn artists(tracks: &[Track], search: &str, sort: GroupSort) -> Vec<ArtistRow> {
    let mut rows: Vec<ArtistRow> = vec![];
    let mut index: HashMap<String, usize> = HashMap::new();
    for track in tracks {
        let names: Vec<Option<&str>> = if track.artists.is_empty() {
            vec![None]
        } else {
            track
                .artists
                .iter()
                .map(|artist| Some(artist.as_str()))
                .collect()
        };
        for name in names {
            let position = *index.entry(key(name)).or_insert_with(|| {
                rows.push(ArtistRow {
                    name: name.map(String::from),
                    tracks: 0,
                    length: 0.0,
                    cover: None,
                });
                rows.len() - 1
            });
            let row = &mut rows[position];
            row.tracks += 1;
            row.length += track.length;
            if row.cover.is_none() {
                row.cover = cover(track);
            }
        }
    }
    let search = search.trim().to_lowercase();
    if !search.is_empty() {
        rows.retain(|row| contains(row.name.as_deref(), &search));
    }
    match sort {
        GroupSort::MostTracks => {
            rows.sort_by_cached_key(|row| (Reverse(row.tracks), key(row.name.as_deref())))
        }
        _ => rows.sort_by_cached_key(|row| key(row.name.as_deref())),
    }
    rows.sort_by_key(|row| row.name.is_none());
    rows
}

/// The genres of `tracks` whose name contains `search`; a track with several genres counts
/// for each.
pub fn genres(tracks: &[Track], search: &str, sort: GroupSort) -> Vec<GenreRow> {
    let mut groups: Vec<(Option<String>, Vec<&Track>)> = vec![];
    let mut index: HashMap<String, usize> = HashMap::new();
    for track in tracks {
        let names: Vec<Option<&str>> = if track.genres.is_empty() {
            vec![None]
        } else {
            track
                .genres
                .iter()
                .map(|genre| Some(genre.as_str()))
                .collect()
        };
        for name in names {
            let position = *index.entry(key(name)).or_insert_with(|| {
                groups.push((name.map(String::from), vec![]));
                groups.len() - 1
            });
            groups[position].1.push(track);
        }
    }
    let mut rows: Vec<GenreRow> = groups
        .into_iter()
        .map(|(name, tracks)| GenreRow {
            name,
            tracks: count(tracks.len()),
            covers: mosaic(tracks),
        })
        .collect();
    let search = search.trim().to_lowercase();
    if !search.is_empty() {
        rows.retain(|row| contains(row.name.as_deref(), &search));
    }
    match sort {
        GroupSort::MostTracks => {
            rows.sort_by_cached_key(|row| (Reverse(row.tracks), key(row.name.as_deref())))
        }
        _ => rows.sort_by_cached_key(|row| key(row.name.as_deref())),
    }
    rows.sort_by_key(|row| row.name.is_none());
    rows
}

/// Every playlist, by name.
pub fn playlists(catalog: &Catalog) -> Vec<PlaylistRow> {
    let mut rows: Vec<PlaylistRow> = catalog
        .playlists()
        .filter_map(|playlist| playlist_row(catalog, playlist.id))
        .collect();
    rows.sort_by_cached_key(|row| row.name.to_lowercase());
    rows
}

pub fn playlist_row(catalog: &Catalog, id: PlaylistId) -> Option<PlaylistRow> {
    let playlist = catalog.playlist(id)?;
    let tracks = catalog.select(&Query::playlist(catalog, id));
    Some(PlaylistRow {
        id,
        name: playlist.name.clone(),
        rule: playlist.rule.clone(),
        sort: playlist.sort.clone(),
        tracks: count(tracks.len()),
        length: tracks.iter().map(|track| track.length).sum(),
        covers: mosaic(&tracks),
        created: playlist.created,
        modified: playlist.modified,
    })
}

/// How many of `tracks` playlist `id` has: all its tracks for a smart one that matches them.
pub fn playlist_holds(catalog: &Catalog, id: PlaylistId, tracks: &[Locator]) -> u32 {
    let Some(playlist) = catalog.playlist(id) else {
        return 0;
    };
    match playlist.rule {
        None => count(
            tracks
                .iter()
                .filter(|track| playlist.items.contains_key(track))
                .count(),
        ),
        Some(_) => {
            let wanted: HashSet<&Locator> = tracks.iter().collect();
            count(
                catalog
                    .select(&Query {
                        filter: Filter::Playlist(id),
                        sort: vec![],
                    })
                    .iter()
                    .filter(|track| wanted.contains(&track.locator))
                    .count(),
            )
        }
    }
}

/// Each of `roots` with what it holds.
pub fn sources(catalog: &Catalog, roots: &[Locator]) -> Vec<SourceRow> {
    roots
        .iter()
        .map(|root| {
            let listed = catalog.listed(root);
            let mut tracks = 0;
            let mut length = 0.0;
            let mut cover_path = None;
            if let Some(listed) = listed {
                for track in catalog.tracks() {
                    if listed.tracks.contains(&track.locator) {
                        tracks += 1;
                        length += track.length;
                        if cover_path.is_none() {
                            cover_path = cover(track);
                        }
                    }
                }
            }
            SourceRow {
                root: root.clone(),
                name: catalog.library_name(root).map(String::from),
                tracks,
                length,
                cover: cover_path,
                listed: listed.is_some(),
                reachable: listed.is_none_or(|listed| listed.reachable),
            }
        })
        .collect()
}

/// The tracks `root` listed when it was last reached that the library leaves out while it
/// can't be: those not saved for offline.
pub fn missing(catalog: &Catalog, root: &Locator) -> Vec<Locator> {
    let Some(listed) = catalog.listed(root) else {
        return vec![];
    };
    if listed.reachable {
        return vec![];
    }
    let mut missing: Vec<Locator> = listed
        .tracks
        .iter()
        .filter(|track| catalog.track(track).is_none())
        .cloned()
        .collect();
    missing.sort_by_cached_key(|track| track.to_string());
    missing
}

pub fn facets(catalog: &Catalog) -> Facets {
    let mut genres: HashMap<String, (String, u32)> = HashMap::new();
    let mut codecs: HashMap<String, u32> = HashMap::new();
    let mut first_year = None;
    let mut last_year = None;
    for track in catalog.tracks() {
        for genre in &track.genres {
            genres
                .entry(genre.to_lowercase())
                .or_insert_with(|| (genre.clone(), 0))
                .1 += 1;
        }
        if let Some(codec) = &track.codec {
            *codecs.entry(codec.to_lowercase()).or_default() += 1;
        }
        if let Some(year) = track.year {
            first_year = Some(first_year.map_or(year, |first: i32| first.min(year)));
            last_year = Some(last_year.map_or(year, |last: i32| last.max(year)));
        }
    }
    let ranked = |facets: Vec<Facet>| {
        let mut facets = facets;
        facets.sort_by_cached_key(|facet| (Reverse(facet.tracks), facet.name.to_lowercase()));
        facets
    };
    Facets {
        genres: ranked(
            genres
                .into_values()
                .map(|(name, tracks)| Facet { name, tracks })
                .collect(),
        ),
        codecs: ranked(
            codecs
                .into_iter()
                .map(|(name, tracks)| Facet { name, tracks })
                .collect(),
        ),
        first_year,
        last_year,
    }
}

/// What a library root is called without a name of its own: its folder's name, its
/// playlist's, or Telegram for a chat, which is named after its title as it is added.
pub fn default_name(root: &Locator) -> String {
    match root {
        Locator::Local(path) => Path::new(path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned()),
        Locator::DocumentTree(uri) => tree_folder(uri).unwrap_or_else(|| uri.clone()),
        Locator::TelegramChat(_) => String::from("Telegram"),
        root => root.display_name(),
    }
}

/// The folder a Storage Access Framework tree URI points at, like `Jazz` for
/// `content://…/tree/primary%3AMusic%2FJazz`; the volume's id for a volume's root.
fn tree_folder(uri: &str) -> Option<String> {
    let encoded = uri.split("/tree/").nth(1)?.split('/').next()?;
    let id = percent_decode(encoded);
    let (volume, path) = id.split_once(':').unwrap_or(("", &id));
    let folder = path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    Some(if folder.is_empty() { volume } else { folder }.to_string())
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let escaped = (bytes[index] == b'%')
            .then(|| bytes.get(index + 1..index + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok());
        match escaped {
            Some(byte) => {
                decoded.push(byte);
                index += 3;
            }
            None => {
                decoded.push(bytes[index]);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// What kind of source a library root is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum SourceType {
    /// A folder; on Android, a document tree too.
    Folder,
    /// A web playlist.
    Web,
    /// A Telegram chat.
    Telegram,
}

/// What is wrong with a source, or happening to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum SourceStatus {
    /// Up to date.
    Ready,
    /// Scanned, or waiting for a scan: "Updating…".
    Updating,
    /// A folder that is gone: "Not found".
    Missing,
    /// A playlist or chat the last scan could not reach: "Can't be reached".
    Unreachable,
    /// A Telegram chat while not signed in: "Signed out of Telegram".
    SignedOut,
}

/// A library root as the desktop's Sources page shows it.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SourceInfo {
    pub root: Locator,
    /// Where it is, as the library writes it: the value of a `source` rule.
    pub location: String,
    pub kind: SourceType,
    /// What it is called: `display_name`, else `default_name`.
    pub name: String,
    /// The name it was given.
    pub display_name: Option<String>,
    /// Its folder's or playlist's name, or Telegram for a chat.
    pub default_name: String,
    /// The tracks of the library it listed.
    pub tracks: u32,
    pub length: f64,
    /// The cover of the first of them with one.
    pub cover: Option<String>,
    /// Up to four covers of its tracks, each once, for a mosaic.
    pub covers: Vec<String>,
    /// False when the folder is gone, or the playlist or chat could not be reached.
    pub available: bool,
    /// Scanned, or waiting for a scan.
    pub updating: bool,
    pub status: SourceStatus,
}

/// A tag the filter rules suggest values of as they are typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum ValueField {
    Artist,
    AlbumArtist,
    Album,
    Genre,
    Codec,
}

/// What a track list holds, for its header.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ListSummary {
    /// The tracks listed.
    pub tracks: u32,
    /// The tracks of the whole library: "%1 of %2 tracks" when fewer are listed.
    pub total: u32,
    /// In seconds.
    pub length: f64,
    /// Up to four covers of the listed tracks, each once, in their order: a mosaic once there
    /// are four.
    pub covers: Vec<String>,
}

pub fn source_type(root: &Locator) -> SourceType {
    match root {
        Locator::Web(_) => SourceType::Web,
        Locator::TelegramChat(_) | Locator::TelegramAudio { .. } => SourceType::Telegram,
        Locator::Local(_) | Locator::DocumentTree(_) | Locator::Document { .. } => {
            SourceType::Folder
        }
    }
}

/// Up to four covers of `tracks`, each once.
pub fn first_covers<'a>(tracks: impl IntoIterator<Item = &'a Track>) -> Vec<String> {
    let mut covers = vec![];
    for path in tracks.into_iter().filter_map(cover) {
        if !covers.contains(&path) {
            covers.push(path);
            if covers.len() == MOSAIC {
                break;
            }
        }
    }
    covers
}

/// `tracks`, of `total` in the library, for a list's header.
pub fn list_summary(tracks: &[Track], total: usize) -> ListSummary {
    ListSummary {
        tracks: count(tracks.len()),
        total: count(total),
        length: tracks.iter().map(|track| track.length).sum(),
        covers: first_covers(tracks),
    }
}

/// `root` with what it holds. `updating` while it is scanned; `signed_in` to Telegram.
pub fn source_info(
    catalog: &Catalog,
    root: &Locator,
    updating: bool,
    signed_in: bool,
) -> SourceInfo {
    let listed = catalog.listed(root);
    let mut tracks = vec![];
    if let Some(listed) = listed {
        tracks.extend(
            catalog
                .tracks()
                .iter()
                .filter(|track| listed.tracks.contains(&track.locator)),
        );
    }
    let kind = source_type(root);
    let available = match root {
        Locator::Local(path) => Path::new(path).is_dir(),
        // Unknown before its first listing.
        Locator::Web(_) | Locator::TelegramChat(_) | Locator::TelegramAudio { .. } => {
            listed.is_none_or(|listed| listed.reachable)
        }
        Locator::DocumentTree(_) | Locator::Document { .. } => true,
    };
    let status = match (available, kind) {
        (false, SourceType::Telegram) if !signed_in => SourceStatus::SignedOut,
        (false, SourceType::Folder) => SourceStatus::Missing,
        (false, _) => SourceStatus::Unreachable,
        (true, _) if updating => SourceStatus::Updating,
        (true, _) => SourceStatus::Ready,
    };
    let display_name = catalog.library_name(root).map(String::from);
    let default_name = default_name(root);
    SourceInfo {
        root: root.clone(),
        location: root.to_string(),
        kind,
        name: display_name.clone().unwrap_or_else(|| default_name.clone()),
        display_name,
        default_name,
        tracks: count(tracks.len()),
        length: tracks.iter().map(|track| track.length).sum(),
        cover: tracks.iter().find_map(|track| cover(track)),
        covers: first_covers(tracks.iter().copied()),
        available,
        updating,
        status,
    }
}

/// The library's values of `field`, each once whatever its case, sorted.
pub fn field_values(catalog: &Catalog, field: ValueField) -> Vec<String> {
    // Keyed by lower case, so `J-Pop` and `j-pop` show once.
    let mut values = std::collections::BTreeMap::<String, String>::new();
    let mut add = |value: &str| {
        let value = value.trim();
        if !value.is_empty() {
            values
                .entry(value.to_lowercase())
                .or_insert_with(|| value.to_string());
        }
    };
    for track in catalog.tracks() {
        match field {
            ValueField::Artist => track.artists.iter().for_each(|artist| add(artist)),
            ValueField::AlbumArtist => track.album_artist.iter().for_each(|artist| add(artist)),
            ValueField::Album => track.album.iter().for_each(|album| add(album)),
            ValueField::Genre => track.genres.iter().for_each(|genre| add(genre)),
            ValueField::Codec => track.codec.iter().for_each(|codec| add(codec)),
        }
    }
    values.into_values().collect()
}

/// The playlists holding `track` as a track added to them, in the library's order.
pub fn playlists_with(catalog: &Catalog, track: &Locator) -> Vec<PlaylistId> {
    catalog
        .playlists()
        .filter(|playlist| playlist.items.contains_key(track))
        .map(|playlist| playlist.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use n_music_core::TrackInfo;
    use std::sync::Arc;

    fn track(path: &str, artists: &[&str], album: Option<&str>, genres: &[&str]) -> Track {
        let mut track = TrackInfo::placeholder(Locator::Local(path.to_string()));
        track.artists = artists.iter().map(|artist| artist.to_string()).collect();
        track.album = album.map(String::from);
        track.genres = genres.iter().map(|genre| genre.to_string()).collect();
        track.length = 60.0;
        Arc::new(track)
    }

    #[test]
    fn albums_group_by_name_and_artist_and_keep_the_untagged_last() {
        let mut tagged = TrackInfo::clone(&track("/a/3", &["Mira"], Some("Paper"), &[]));
        tagged.year = Some(2019);
        let tracks = vec![
            track("/a/1", &["Kōsuke"], Some("Blue Hour"), &[]),
            track("/a/2", &["kōsuke"], Some("blue hour"), &[]),
            Arc::new(tagged),
            track("/a/4", &[], None, &[]),
            track("/a/5", &["Ada"], Some("Blue Hour"), &[]),
        ];
        let rows = albums(&tracks, "", GroupSort::Name);
        let names: Vec<_> = rows
            .iter()
            .map(|row| (row.name.as_deref(), row.artist.as_deref(), row.tracks))
            .collect();
        assert_eq!(
            names,
            [
                (Some("Blue Hour"), Some("Kōsuke"), 2),
                (Some("Blue Hour"), Some("Ada"), 1),
                (Some("Paper"), Some("Mira"), 1),
                (None, None, 1),
            ]
        );
        assert_eq!(rows[0].length, 120.0);
        assert_eq!(rows[2].year, Some(2019));

        let by_artist = albums(&tracks, "", GroupSort::Artist);
        assert_eq!(by_artist[0].artist.as_deref(), Some("Ada"));
        let found = albums(&tracks, "mir", GroupSort::Name);
        assert_eq!(found.len(), 1, "an album is found by its artist too");
    }

    #[test]
    fn a_track_counts_for_each_of_its_artists_and_genres() {
        let tracks = vec![
            track("/b/1", &["Ada", "Mira"], None, &["Jazz", "Blues"]),
            track("/b/2", &["Mira"], None, &["jazz"]),
            track("/b/3", &[], None, &[]),
        ];
        let artists = artists(&tracks, "", GroupSort::MostTracks);
        let names: Vec<_> = artists
            .iter()
            .map(|row| (row.name.as_deref(), row.tracks))
            .collect();
        assert_eq!(names, [(Some("Mira"), 2), (Some("Ada"), 1), (None, 1)]);

        let genres = genres(&tracks, "blue", GroupSort::Name);
        assert_eq!(genres.len(), 1);
        assert_eq!(genres[0].name.as_deref(), Some("Blues"));
    }

    #[test]
    fn sources_are_named_after_their_folder() {
        assert_eq!(
            default_name(&Locator::DocumentTree(String::from(
                "content://com.android.externalstorage.documents/tree/primary%3AMusic%2FJazz"
            ))),
            "Jazz"
        );
        assert_eq!(
            default_name(&Locator::DocumentTree(String::from(
                "content://com.android.externalstorage.documents/tree/1234-ABCD%3A"
            ))),
            "1234-ABCD"
        );
        assert_eq!(
            default_name(&Locator::Local(String::from("/home/me/Music"))),
            "Music"
        );
        assert_eq!(
            default_name(&Locator::Web(String::from("https://example.net/live.pls"))),
            "live"
        );
    }
}
