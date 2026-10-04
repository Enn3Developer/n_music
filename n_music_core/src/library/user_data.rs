//! What the user creates, in the library database: playlists and play statistics. Unlike
//! scanned rows, none of it can be rebuilt.

use super::catalog::{PlayStats, Playlist, PlaylistItem};
use super::db::{decode_locator, encode_locator, LibraryDb, Result};
use super::query::{Filter, PlaylistId, SortKey};
use crate::source::Locator;
use rusqlite::{params, Transaction};
use std::collections::HashMap;

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
