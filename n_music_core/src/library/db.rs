//! The library database (SQLite). Scanned track metadata lives here; rows are keyed by locator
//! and trusted only while the provider's version stamp and the reader's format still match.
//! It also links each library to the tracks it lists: a track no library lists is forgotten.
//! And it keeps the names libraries were given.

use super::track::{ReplayGain, TrackInfo};
use crate::source::Locator;
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction, TransactionBehavior};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub use rusqlite::Error;
pub type Result<T> = rusqlite::Result<T>;

/// Schema migrations, applied in order; `PRAGMA user_version` counts the applied ones.
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/001_tracks.sql"),
    include_str!("migrations/002_user_data.sql"),
    include_str!("migrations/003_drop_session.sql"),
    include_str!("migrations/004_session.sql"),
    include_str!("migrations/005_library_tracks.sql"),
    include_str!("migrations/006_library_names.sql"),
];

/// Bump whenever [`super::reader::read_info`] reads more or differently: rows
/// written by an older reader are then read again on the next scan.
pub const FORMAT: i64 = 2;

const KIND_LOCAL: i64 = 0;
const KIND_DOCUMENT: i64 = 1;
const KIND_WEB: i64 = 2;
/// Only of libraries.
const KIND_TREE: i64 = 3;

/// A track as last scanned.
pub struct StoredTrack {
    pub version: u64,
    /// `None` when the file could not be read.
    pub info: Option<TrackInfo>,
}

/// A scan result to store.
pub struct ScannedTrack<'a> {
    pub locator: &'a Locator,
    pub version: u64,
    pub info: Option<&'a TrackInfo>,
}

/// The tracks a scan listed of a library.
pub struct LibraryTracks<'a> {
    pub library: &'a Locator,
    pub tracks: Vec<&'a Locator>,
    /// It could be listed: these replace the tracks it listed before. One that could not keeps
    /// those, gaining these, which play offline.
    pub reachable: bool,
}

pub struct LibraryDb {
    pub(super) conn: Connection,
    covers: PathBuf,
}

impl LibraryDb {
    /// Opens (creating and migrating if needed) the database at `path`. Cover file names are
    /// resolved inside `covers`.
    pub fn open(path: &Path, covers: &Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        migrate(&mut conn)?;
        Ok(Self {
            conn,
            covers: covers.to_path_buf(),
        })
    }

    /// Every track scanned by the current [`FORMAT`].
    pub fn tracks(&self) -> Result<HashMap<Locator, StoredTrack>> {
        let mut artists = self.values("track_artists")?;
        let mut genres = self.values("track_genres")?;
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM tracks WHERE format = ?1"))?;
        let rows = statement.query_map([FORMAT], |row| self.read_row(row))?;
        let mut tracks = HashMap::new();
        for row in rows {
            let Some(row) = row? else {
                continue;
            };
            let info = row.info.map(|mut info| {
                info.artists = artists.remove(&row.id).unwrap_or_default();
                info.genres = genres.remove(&row.id).unwrap_or_default();
                info
            });
            tracks.insert(
                row.locator,
                StoredTrack {
                    version: row.version,
                    info,
                },
            );
        }
        Ok(tracks)
    }

    /// The stored metadata of one readable track, whatever its version.
    pub fn track(&self, locator: &Locator) -> Result<Option<TrackInfo>> {
        let Some((kind, location, _)) = encode_locator(locator) else {
            return Ok(None);
        };
        let row = self
            .conn
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM tracks WHERE kind = ?1 AND location = ?2 AND format = ?3"
                ),
                params![kind, location, FORMAT],
                |row| self.read_row(row),
            )
            .optional()?
            .flatten();
        let Some(StoredRow {
            id,
            info: Some(mut info),
            ..
        }) = row
        else {
            return Ok(None);
        };
        info.artists = self.track_values("track_artists", id)?;
        info.genres = self.track_values("track_genres", id)?;
        Ok(Some(info))
    }

    /// Inserts or replaces `tracks` in one transaction.
    pub fn save(&mut self, tracks: &[ScannedTrack]) -> Result<()> {
        let transaction = self.conn.transaction()?;
        for track in tracks {
            save_track(&transaction, track)?;
        }
        transaction.commit()
    }

    /// Deletes every track not in `keep`; returns how many were deleted.
    pub fn retain(&mut self, keep: &HashSet<Locator>) -> Result<usize> {
        let transaction = self.conn.transaction()?;
        let stale = {
            let mut statement =
                transaction.prepare("SELECT id, kind, location, name FROM tracks")?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    decode_locator(row.get(1)?, row.get(2)?, row.get(3)?),
                ))
            })?;
            let mut stale = vec![];
            for row in rows {
                let (id, locator) = row?;
                if locator.is_none_or(|locator| !keep.contains(&locator)) {
                    stale.push(id);
                }
            }
            stale
        };
        {
            let mut delete = transaction.prepare("DELETE FROM tracks WHERE id = ?1")?;
            for id in &stale {
                delete.execute([id])?;
            }
        }
        transaction.commit()?;
        Ok(stale.len())
    }

    /// The tracks each of `libraries` listed when last listed; one never listed since the
    /// database links tracks is left out.
    pub fn library_tracks(&self, libraries: &[Locator]) -> Result<HashMap<Locator, Vec<Locator>>> {
        let mut found = HashMap::new();
        for library in libraries {
            let Some((_, location)) = encode_library(library) else {
                continue;
            };
            let id: Option<i64> = self
                .conn
                .query_row(
                    "SELECT id FROM libraries WHERE location = ?1 AND listed",
                    [location],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(id) = id else {
                continue;
            };
            let tracks = linked(&self.conn, id)?
                .into_iter()
                .filter_map(|(location, (kind, name))| decode_locator(kind, location, name))
                .collect();
            found.insert(library.clone(), tracks);
        }
        Ok(found)
    }

    /// Links each library of `listed` to its tracks and forgets those not in `libraries`, the
    /// libraries there are, names and all. Then forgets the tracks unlinked that no library
    /// lists; returns how many. One that could not be listed is linked once one could: until
    /// then, what it lists is unknown.
    ///
    /// `libraries` is asked once no other connection can write: a library added meanwhile, and
    /// named at once, keeps its name.
    pub fn link(
        &mut self,
        listed: &[LibraryTracks],
        libraries: impl FnOnce() -> Vec<Locator>,
    ) -> Result<usize> {
        // Reading the links, then writing them, must not let another connection write between.
        let transaction = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let libraries = libraries();
        let mut unlinked = HashSet::new();
        let removed = {
            let mut statement = transaction.prepare("SELECT id, kind, location FROM libraries")?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    decode_library(row.get(1)?, row.get(2)?),
                ))
            })?;
            let mut removed = vec![];
            for row in rows {
                let (id, library) = row?;
                if library.is_none_or(|library| !libraries.contains(&library)) {
                    removed.push(id);
                }
            }
            removed
        };
        for id in removed {
            unlinked.extend(linked(&transaction, id)?.into_keys());
            transaction.execute("DELETE FROM libraries WHERE id = ?1", [id])?;
        }
        for listing in listed {
            let Some((kind, location)) =
                encode_library(listing.library).filter(|_| libraries.contains(listing.library))
            else {
                continue;
            };
            let id: Option<i64> = if listing.reachable {
                Some(transaction.query_row(
                    "INSERT INTO libraries (kind, location) VALUES (?1, ?2) \
                     ON CONFLICT (location) DO UPDATE SET kind = excluded.kind, listed = 1 \
                     RETURNING id",
                    params![kind, location],
                    |row| row.get(0),
                )?)
            } else {
                transaction
                    .query_row(
                        "SELECT id FROM libraries WHERE location = ?1 AND listed",
                        [location],
                        |row| row.get(0),
                    )
                    .optional()?
            };
            let Some(id) = id else {
                continue;
            };
            let old = linked(&transaction, id)?;
            let new: HashMap<&str, (i64, Option<&str>)> = listing
                .tracks
                .iter()
                .copied()
                .filter_map(encode_locator)
                .map(|(kind, location, name)| (location, (kind, name)))
                .collect();
            if listing.reachable {
                let mut unlink = transaction.prepare_cached(
                    "DELETE FROM library_tracks WHERE library_id = ?1 AND location = ?2",
                )?;
                for location in old
                    .keys()
                    .filter(|location| !new.contains_key(location.as_str()))
                {
                    unlink.execute(params![id, location])?;
                    unlinked.insert(location.clone());
                }
            }
            let mut link = transaction.prepare_cached(
                "INSERT INTO library_tracks (library_id, kind, location, name) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT (library_id, location) DO UPDATE SET kind = excluded.kind, \
                     name = excluded.name",
            )?;
            for (location, (kind, name)) in &new {
                let known = old.get(*location).is_some_and(|(known_kind, known_name)| {
                    known_kind == kind && known_name.as_deref() == *name
                });
                if !known {
                    link.execute(params![id, kind, location, name])?;
                }
            }
        }
        let mut forgotten = 0;
        {
            let mut forget = transaction.prepare(
                "DELETE FROM tracks WHERE location = ?1 \
                 AND NOT EXISTS (SELECT 1 FROM library_tracks WHERE location = ?1)",
            )?;
            for location in &unlinked {
                forgotten += forget.execute([location])?;
            }
        }
        transaction.commit()?;
        Ok(forgotten)
    }

    /// The names libraries were given.
    pub fn library_names(&self) -> Result<HashMap<Locator, String>> {
        let mut statement = self.conn.prepare(
            "SELECT kind, location, display_name FROM libraries WHERE display_name IS NOT NULL",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((decode_library(row.get(0)?, row.get(1)?), row.get(2)?))
        })?;
        let mut names = HashMap::new();
        for row in rows {
            if let (Some(library), name) = row? {
                names.insert(library, name);
            }
        }
        Ok(names)
    }

    /// Calls `library` `name`, or (`None`) by its folder's or playlist's name again.
    pub fn name_library(&mut self, library: &Locator, name: Option<&str>) -> Result<()> {
        let Some((kind, location)) = encode_library(library) else {
            return Ok(());
        };
        match name {
            // One not listed yet gets a row for its name alone.
            Some(name) => self.conn.execute(
                "INSERT INTO libraries (kind, location, display_name, listed) \
                 VALUES (?1, ?2, ?3, 0) \
                 ON CONFLICT (location) DO UPDATE SET display_name = excluded.display_name",
                params![kind, location, name],
            )?,
            None => {
                self.conn.execute(
                    "DELETE FROM libraries WHERE location = ?1 AND NOT listed",
                    [location],
                )?;
                self.conn.execute(
                    "UPDATE libraries SET display_name = NULL WHERE location = ?1",
                    [location],
                )?
            }
        };
        Ok(())
    }

    /// File names of every cover a track refers to.
    pub fn cover_names(&self) -> Result<HashSet<String>> {
        let mut statement = self
            .conn
            .prepare("SELECT DISTINCT cover FROM tracks WHERE cover IS NOT NULL")?;
        let names = statement.query_map([], |row| row.get(0))?;
        names.collect()
    }

    /// `None` for a row of an unknown locator kind.
    fn read_row(&self, row: &Row) -> Result<Option<StoredRow>> {
        let id: i64 = row.get(0)?;
        let version = row.get::<_, i64>(4)? as u64;
        let Some(locator) = decode_locator(row.get(1)?, row.get(2)?, row.get(3)?) else {
            return Ok(None);
        };
        if !row.get::<_, bool>(5)? {
            return Ok(Some(StoredRow {
                id,
                locator,
                version,
                info: None,
            }));
        }
        let info = TrackInfo {
            title: row
                .get::<_, Option<String>>(6)?
                .unwrap_or_else(|| locator.display_name()),
            locator: locator.clone(),
            artists: vec![],
            album: row.get(7)?,
            album_artist: row.get(8)?,
            track_number: row.get(9)?,
            track_total: row.get(10)?,
            disc_number: row.get(11)?,
            disc_total: row.get(12)?,
            year: row.get(13)?,
            genres: vec![],
            length: row.get(14)?,
            codec: row.get(15)?,
            sample_rate: row.get(16)?,
            channels: row.get(17)?,
            bits_per_sample: row.get(18)?,
            replay_gain: ReplayGain {
                track_gain: row.get(19)?,
                track_peak: row.get(20)?,
                album_gain: row.get(21)?,
                album_peak: row.get(22)?,
            },
            cover: row
                .get::<_, Option<String>>(23)?
                .map(|name| self.covers.join(name)),
            fingerprint: row.get::<_, Option<i64>>(24)?.map(|value| value as u64),
        };
        Ok(Some(StoredRow {
            id,
            locator,
            version,
            info: Some(info),
        }))
    }

    /// All rows of a multi-valued field table, grouped by track and ordered by position.
    fn values(&self, table: &str) -> Result<HashMap<i64, Vec<String>>> {
        let mut statement = self.conn.prepare(&format!(
            "SELECT track_id, name FROM {table} ORDER BY track_id, position"
        ))?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut values: HashMap<i64, Vec<String>> = HashMap::new();
        for row in rows {
            let (id, name) = row?;
            values.entry(id).or_default().push(name);
        }
        Ok(values)
    }

    fn track_values(&self, table: &str, id: i64) -> Result<Vec<String>> {
        let mut statement = self.conn.prepare(&format!(
            "SELECT name FROM {table} WHERE track_id = ?1 ORDER BY position"
        ))?;
        let names = statement.query_map([id], |row| row.get(0))?;
        names.collect()
    }
}

/// Columns read by [`LibraryDb::read_row`], in order.
const COLUMNS: &str = "id, kind, location, name, version, readable, title, album, album_artist, \
    track_number, track_total, disc_number, disc_total, year, length, codec, sample_rate, \
    channels, bits_per_sample, track_gain, track_peak, album_gain, album_peak, cover, fingerprint";

struct StoredRow {
    id: i64,
    locator: Locator,
    version: u64,
    info: Option<TrackInfo>,
}

fn save_track(transaction: &Transaction, track: &ScannedTrack) -> Result<()> {
    let Some((kind, location, name)) = encode_locator(track.locator) else {
        return Ok(());
    };
    let info = track.info;
    let gain = info.map(|info| &info.replay_gain);
    let cover = info
        .and_then(|info| info.cover.as_deref())
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned());
    let id: i64 = transaction
        .prepare_cached(
            "INSERT INTO tracks (kind, location, name, version, format, readable, title, album, \
                 album_artist, track_number, track_total, disc_number, disc_total, year, length, \
                 codec, sample_rate, channels, bits_per_sample, track_gain, track_peak, \
                 album_gain, album_peak, cover, fingerprint) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, \
                 ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25) \
             ON CONFLICT (location) DO UPDATE SET kind = excluded.kind, name = excluded.name, \
                 version = excluded.version, format = excluded.format, \
                 readable = excluded.readable, title = excluded.title, album = excluded.album, \
                 album_artist = excluded.album_artist, track_number = excluded.track_number, \
                 track_total = excluded.track_total, disc_number = excluded.disc_number, \
                 disc_total = excluded.disc_total, year = excluded.year, \
                 length = excluded.length, codec = excluded.codec, \
                 sample_rate = excluded.sample_rate, channels = excluded.channels, \
                 bits_per_sample = excluded.bits_per_sample, track_gain = excluded.track_gain, \
                 track_peak = excluded.track_peak, album_gain = excluded.album_gain, \
                 album_peak = excluded.album_peak, cover = excluded.cover, \
                 fingerprint = excluded.fingerprint \
             RETURNING id",
        )?
        .query_row(
            params![
                kind,
                location,
                name,
                track.version as i64,
                FORMAT,
                info.is_some(),
                info.map(|info| &info.title),
                info.and_then(|info| info.album.as_ref()),
                info.and_then(|info| info.album_artist.as_ref()),
                info.and_then(|info| info.track_number),
                info.and_then(|info| info.track_total),
                info.and_then(|info| info.disc_number),
                info.and_then(|info| info.disc_total),
                info.and_then(|info| info.year),
                info.map_or(0.0, |info| info.length),
                info.and_then(|info| info.codec.as_ref()),
                info.and_then(|info| info.sample_rate),
                info.and_then(|info| info.channels),
                info.and_then(|info| info.bits_per_sample),
                gain.and_then(|gain| gain.track_gain),
                gain.and_then(|gain| gain.track_peak),
                gain.and_then(|gain| gain.album_gain),
                gain.and_then(|gain| gain.album_peak),
                cover,
                info.and_then(|info| info.fingerprint)
                    .map(|value| value as i64),
            ],
            |row| row.get(0),
        )?;
    let empty = vec![];
    save_values(
        transaction,
        "track_artists",
        id,
        info.map_or(&empty, |info| &info.artists),
    )?;
    save_values(
        transaction,
        "track_genres",
        id,
        info.map_or(&empty, |info| &info.genres),
    )
}

fn save_values(transaction: &Transaction, table: &str, id: i64, values: &[String]) -> Result<()> {
    transaction
        .prepare_cached(&format!("DELETE FROM {table} WHERE track_id = ?1"))?
        .execute([id])?;
    let mut insert = transaction.prepare_cached(&format!(
        "INSERT INTO {table} (track_id, position, name) VALUES (?1, ?2, ?3)"
    ))?;
    for (position, value) in values.iter().enumerate() {
        insert.execute(params![id, position as i64, value])?;
    }
    Ok(())
}

/// The tracks a library links, by location, with their kind and name.
fn linked(conn: &Connection, id: i64) -> Result<HashMap<String, (i64, Option<String>)>> {
    let mut statement = conn
        .prepare_cached("SELECT location, kind, name FROM library_tracks WHERE library_id = ?1")?;
    let rows = statement.query_map([id], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))?;
    rows.collect()
}

/// `None` for a document, which is no library.
fn encode_library(library: &Locator) -> Option<(i64, &str)> {
    match library {
        Locator::Local(path) => Some((KIND_LOCAL, path)),
        Locator::Web(address) => Some((KIND_WEB, address)),
        Locator::DocumentTree(uri) => Some((KIND_TREE, uri)),
        Locator::Document { .. } => None,
    }
}

fn decode_library(kind: i64, location: String) -> Option<Locator> {
    match kind {
        KIND_LOCAL => Some(Locator::Local(location)),
        KIND_WEB => Some(Locator::Web(location)),
        KIND_TREE => Some(Locator::DocumentTree(location)),
        _ => None,
    }
}

pub(super) fn encode_locator(locator: &Locator) -> Option<(i64, &str, Option<&str>)> {
    match locator {
        Locator::Local(path) => Some((KIND_LOCAL, path, None)),
        Locator::Document { uri, name } => Some((KIND_DOCUMENT, uri, Some(name))),
        Locator::Web(address) => Some((KIND_WEB, address, None)),
        Locator::DocumentTree(_) => None,
    }
}

pub(super) fn decode_locator(kind: i64, location: String, name: Option<String>) -> Option<Locator> {
    match kind {
        KIND_LOCAL => Some(Locator::Local(location)),
        KIND_DOCUMENT => Some(Locator::Document {
            uri: location,
            name: name.unwrap_or_default(),
        }),
        KIND_WEB => Some(Locator::Web(location)),
        _ => None,
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let applied: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let applied = usize::try_from(applied).unwrap_or(usize::MAX);
    if applied > MIGRATIONS.len() {
        return Err(Error::InvalidParameterName(format!(
            "library schema {applied} is newer than this version of N Music ({})",
            MIGRATIONS.len()
        )));
    }
    if applied == MIGRATIONS.len() {
        return Ok(());
    }
    let transaction = conn.transaction()?;
    for (index, migration) in MIGRATIONS.iter().enumerate().skip(applied) {
        log::info!("Applying library migration {}", index + 1);
        transaction.execute_batch(migration)?;
    }
    transaction.pragma_update(None, "user_version", MIGRATIONS.len() as i64)?;
    transaction.commit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(path: &str) -> TrackInfo {
        let mut info = TrackInfo::placeholder(Locator::Local(path.into()));
        info.artists = vec!["A".into(), "B".into()];
        info.genres = vec!["Rock".into()];
        info.album = Some("Album".into());
        info.track_number = Some(2);
        info.length = 120.5;
        info.replay_gain.track_gain = Some(-3.5);
        info.cover = Some(PathBuf::from("/covers/abc.jpg"));
        info
    }

    fn open(dir: &Path) -> LibraryDb {
        LibraryDb::open(&dir.join("library.db"), Path::new("/covers")).unwrap()
    }

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = open(dir.path());
        let a = info("a");
        let broken = Locator::Local("broken".into());
        db.save(&[
            ScannedTrack {
                locator: &a.locator,
                version: 1,
                info: Some(&a),
            },
            ScannedTrack {
                locator: &broken,
                version: 7,
                info: None,
            },
        ])
        .unwrap();
        drop(db);

        let db = open(dir.path());
        let tracks = db.tracks().unwrap();
        let stored = &tracks[&a.locator];
        assert_eq!(stored.version, 1);
        assert_eq!(stored.info.as_ref(), Some(&a));
        assert_eq!(tracks[&broken].version, 7);
        assert!(tracks[&broken].info.is_none());
        assert_eq!(db.track(&a.locator).unwrap(), Some(a));
        assert_eq!(db.cover_names().unwrap(), HashSet::from(["abc.jpg".into()]));
    }

    #[test]
    fn update_replaces_values_and_retain_deletes() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = open(dir.path());
        let mut a = info("a");
        let b = info("b");
        let save = |db: &mut LibraryDb, track: &TrackInfo| {
            db.save(&[ScannedTrack {
                locator: &track.locator,
                version: 1,
                info: Some(track),
            }])
            .unwrap()
        };
        save(&mut db, &a);
        save(&mut db, &b);
        a.artists = vec!["C".into()];
        save(&mut db, &a);
        assert_eq!(db.track(&a.locator).unwrap().unwrap().artists, ["C"]);

        let deleted = db.retain(&HashSet::from([a.locator.clone()])).unwrap();
        assert_eq!(deleted, 1);
        assert!(db.track(&b.locator).unwrap().is_none());
        let artists: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM track_artists", [], |row| row.get(0))
            .unwrap();
        assert_eq!(artists, 1);
    }
}
