//! What the user creates, in the library database: playlists, play statistics and the play
//! session. Unlike scanned rows, none of it can be rebuilt.

use super::catalog::{PlayStats, Playlist, PlaylistItem};
use super::db::{decode_locator, encode_locator, LibraryDb, Result};
use super::query::{Filter, PlaylistId, Query, SortKey};
use crate::source::Locator;
use rusqlite::{params, OptionalExtension, Transaction};
use std::collections::HashMap;

/// A track the session refers to.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredItem {
    pub locator: Locator,
    pub fingerprint: Option<u64>,
}

/// Where the session is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StoredCursor {
    /// The context item at this position.
    Context(usize),
    /// The up-next item playing now.
    Detour,
}

/// The session's tracks; saved whenever they change.
#[derive(Clone, Debug, Default)]
pub struct SessionItems {
    pub context: Option<Query>,
    /// In context order.
    pub items: Vec<StoredItem>,
    /// Each item's place in the play order.
    pub slots: Vec<usize>,
    pub up_next: Vec<StoredItem>,
    pub detour: Option<StoredItem>,
}

/// Where the session is; saved often.
#[derive(Clone, Copy, Debug, Default)]
pub struct SessionState {
    pub cursor: Option<StoredCursor>,
    /// Seconds into the current item.
    pub position: f64,
    pub finished: bool,
}

const LIST_CONTEXT: i64 = 0;
const LIST_UP_NEXT: i64 = 1;
const LIST_DETOUR: i64 = 2;

impl LibraryDb {
    /// Every playlist with its items.
    pub fn playlists(&self) -> Result<Vec<Playlist>> {
        let mut items: HashMap<i64, HashMap<Locator, PlaylistItem>> = HashMap::new();
        {
            let mut statement = self.conn.prepare(
                "SELECT playlist_id, kind, location, name, fingerprint, added FROM playlist_items",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    decode_locator(row.get(1)?, row.get(2)?, row.get(3)?),
                    PlaylistItem {
                        fingerprint: row.get::<_, Option<i64>>(4)?.map(|value| value as u64),
                        added: row.get(5)?,
                    },
                ))
            })?;
            for row in rows {
                if let (id, Some(locator), item) = row? {
                    items.entry(id).or_default().insert(locator, item);
                }
            }
        }
        let mut statement = self
            .conn
            .prepare("SELECT id, name, rule, sort, created, modified FROM playlists")?;
        let rows = statement.query_map([], |row| {
            let id: i64 = row.get(0)?;
            Ok(Playlist {
                id: PlaylistId(id),
                name: row.get(1)?,
                rule: row
                    .get::<_, Option<String>>(2)?
                    .and_then(|rule| from_json(&rule)),
                sort: from_json(&row.get::<_, String>(3)?).unwrap_or_default(),
                items: items.remove(&id).unwrap_or_default(),
                created: row.get(4)?,
                modified: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub fn create_playlist(
        &mut self,
        name: &str,
        rule: Option<&Filter>,
        sort: &[SortKey],
        now: i64,
    ) -> Result<PlaylistId> {
        self.conn
            .execute(
                "INSERT INTO playlists (name, rule, sort, created, modified) \
                 VALUES (?1, ?2, ?3, ?4, ?4)",
                params![name, rule.map(to_json), to_json(&sort), now],
            )
            .map(|_| PlaylistId(self.conn.last_insert_rowid()))
    }

    /// Saves a playlist's name, rule and sort; its items are saved separately.
    pub fn update_playlist(&mut self, playlist: &Playlist) -> Result<()> {
        self.conn.execute(
            "UPDATE playlists SET name = ?2, rule = ?3, sort = ?4, modified = ?5 WHERE id = ?1",
            params![
                playlist.id.0,
                playlist.name,
                playlist.rule.as_ref().map(to_json),
                to_json(&playlist.sort),
                playlist.modified,
            ],
        )?;
        Ok(())
    }

    pub fn delete_playlist(&mut self, id: PlaylistId) -> Result<()> {
        self.conn
            .execute("DELETE FROM playlists WHERE id = ?1", [id.0])?;
        Ok(())
    }

    /// Adds tracks to a playlist; tracks already in it keep their date.
    pub fn add_to_playlist(
        &mut self,
        id: PlaylistId,
        tracks: &[(Locator, PlaylistItem)],
        now: i64,
    ) -> Result<()> {
        let transaction = self.conn.transaction()?;
        {
            let mut insert = transaction.prepare(
                "INSERT OR IGNORE INTO playlist_items \
                 (playlist_id, kind, location, name, fingerprint, added) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for (locator, item) in tracks {
                let Some((kind, location, name)) = encode_locator(locator) else {
                    continue;
                };
                insert.execute(params![
                    id.0,
                    kind,
                    location,
                    name,
                    item.fingerprint.map(|value| value as i64),
                    item.added,
                ])?;
            }
            touch(&transaction, id, now)?;
        }
        transaction.commit()
    }

    pub fn remove_from_playlist(
        &mut self,
        id: PlaylistId,
        tracks: &[Locator],
        now: i64,
    ) -> Result<()> {
        let transaction = self.conn.transaction()?;
        {
            let mut delete = transaction
                .prepare("DELETE FROM playlist_items WHERE playlist_id = ?1 AND location = ?2")?;
            for locator in tracks {
                if let Some((_, location, _)) = encode_locator(locator) {
                    delete.execute(params![id.0, location])?;
                }
            }
            touch(&transaction, id, now)?;
        }
        transaction.commit()
    }

    pub fn play_stats(&self) -> Result<HashMap<Locator, PlayStats>> {
        let mut statement = self
            .conn
            .prepare("SELECT kind, location, name, plays, last_played FROM play_stats")?;
        let rows = statement.query_map([], |row| {
            Ok((
                decode_locator(row.get(0)?, row.get(1)?, row.get(2)?),
                PlayStats {
                    plays: row.get(3)?,
                    last_played: row.get(4)?,
                },
            ))
        })?;
        let mut stats = HashMap::new();
        for row in rows {
            if let (Some(locator), row_stats) = row? {
                stats.insert(locator, row_stats);
            }
        }
        Ok(stats)
    }

    pub fn record_play(
        &mut self,
        locator: &Locator,
        fingerprint: Option<u64>,
        now: i64,
    ) -> Result<()> {
        let Some((kind, location, name)) = encode_locator(locator) else {
            return Ok(());
        };
        self.conn.execute(
            "INSERT INTO play_stats (location, kind, name, fingerprint, plays, last_played) \
             VALUES (?1, ?2, ?3, ?4, 1, ?5) \
             ON CONFLICT (location) DO UPDATE SET plays = plays + 1, \
                 last_played = excluded.last_played, \
                 fingerprint = coalesce(excluded.fingerprint, fingerprint)",
            params![
                location,
                kind,
                name,
                fingerprint.map(|value| value as i64),
                now
            ],
        )?;
        Ok(())
    }

    /// Points playlist items and statistics of files that are gone to the scanned track with
    /// the same fingerprint, when there is one: the file was moved or renamed.
    pub fn repoint(&mut self) -> Result<()> {
        let transaction = self.conn.transaction()?;
        for table in ["playlist_items", "play_stats"] {
            transaction.execute(
                &format!(
                    "UPDATE OR IGNORE {table} SET kind = moved.kind, location = moved.location, \
                         name = moved.name \
                     FROM (SELECT fingerprint, kind, location, name FROM tracks \
                         WHERE fingerprint IS NOT NULL GROUP BY fingerprint) AS moved \
                     WHERE {table}.fingerprint = moved.fingerprint \
                         AND NOT EXISTS \
                             (SELECT 1 FROM tracks WHERE tracks.location = {table}.location)"
                ),
                [],
            )?;
        }
        transaction.commit()
    }

    /// The saved session, if any.
    pub fn session(&self) -> Result<Option<(SessionItems, SessionState)>> {
        let Some((context, list, position, offset, finished)) = self
            .conn
            .query_row(
                "SELECT context, current_list, current_position, position, finished \
                 FROM session WHERE id = 0",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, bool>(4)?,
                    ))
                },
            )
            .optional()?
        else {
            return Ok(None);
        };
        let mut items = SessionItems {
            context: context.and_then(|context| from_json(&context)),
            ..SessionItems::default()
        };
        let mut statement = self.conn.prepare(
            "SELECT list, slot, kind, location, name, fingerprint FROM session_items \
             ORDER BY list, position",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?,
                decode_locator(row.get(2)?, row.get(3)?, row.get(4)?),
                row.get::<_, Option<i64>>(5)?.map(|value| value as u64),
            ))
        })?;
        for row in rows {
            let (list, slot, locator, fingerprint) = row?;
            let Some(locator) = locator else {
                continue;
            };
            let item = StoredItem {
                locator,
                fingerprint,
            };
            match list {
                LIST_CONTEXT => {
                    items.items.push(item);
                    items.slots.push(slot.unwrap_or(0) as usize);
                }
                LIST_UP_NEXT => items.up_next.push(item),
                LIST_DETOUR => items.detour = Some(item),
                _ => {}
            }
        }
        let cursor = match (list, position) {
            (Some(LIST_CONTEXT), Some(position)) => Some(StoredCursor::Context(position as usize)),
            (Some(LIST_DETOUR), _) => Some(StoredCursor::Detour),
            _ => None,
        };
        Ok(Some((
            items,
            SessionState {
                cursor,
                position: offset,
                finished,
            },
        )))
    }

    /// Replaces the session's tracks.
    pub fn save_session_items(&mut self, items: &SessionItems) -> Result<()> {
        let transaction = self.conn.transaction()?;
        transaction.execute(
            "INSERT INTO session (id, context) VALUES (0, ?1) \
             ON CONFLICT (id) DO UPDATE SET context = excluded.context",
            [items.context.as_ref().map(to_json)],
        )?;
        transaction.execute("DELETE FROM session_items", [])?;
        {
            let mut insert = transaction.prepare(
                "INSERT INTO session_items \
                 (list, position, slot, kind, location, name, fingerprint) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            let lists = [
                (LIST_CONTEXT, &items.items[..]),
                (LIST_UP_NEXT, &items.up_next[..]),
                (LIST_DETOUR, items.detour.as_slice()),
            ];
            for (list, stored) in lists {
                for (position, item) in stored.iter().enumerate() {
                    let Some((kind, location, name)) = encode_locator(&item.locator) else {
                        continue;
                    };
                    let slot = (list == LIST_CONTEXT)
                        .then(|| items.slots.get(position).map(|&slot| slot as i64))
                        .flatten();
                    insert.execute(params![
                        list,
                        position as i64,
                        slot,
                        kind,
                        location,
                        name,
                        item.fingerprint.map(|value| value as i64),
                    ])?;
                }
            }
        }
        transaction.commit()
    }

    pub fn save_session_state(&mut self, state: &SessionState) -> Result<()> {
        let (list, position) = match state.cursor {
            Some(StoredCursor::Context(position)) => (Some(LIST_CONTEXT), Some(position as i64)),
            Some(StoredCursor::Detour) => (Some(LIST_DETOUR), None),
            None => (None, None),
        };
        self.conn.execute(
            "INSERT INTO session (id, current_list, current_position, position, finished) \
             VALUES (0, ?1, ?2, ?3, ?4) \
             ON CONFLICT (id) DO UPDATE SET current_list = excluded.current_list, \
                 current_position = excluded.current_position, position = excluded.position, \
                 finished = excluded.finished",
            params![list, position, state.position, state.finished],
        )?;
        Ok(())
    }
}

fn touch(transaction: &Transaction, id: PlaylistId, now: i64) -> Result<()> {
    transaction.execute(
        "UPDATE playlists SET modified = ?2 WHERE id = ?1",
        params![id.0, now],
    )?;
    Ok(())
}

fn to_json<T: serde::Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).expect("queries serialize")
}

/// `None`, with a warning, for JSON this version cannot read.
fn from_json<T: serde::de::DeserializeOwned>(json: &str) -> Option<T> {
    serde_json::from_str(json)
        .inspect_err(|error| log::warn!("Ignoring unreadable saved query {json:?}: {error}"))
        .ok()
}
