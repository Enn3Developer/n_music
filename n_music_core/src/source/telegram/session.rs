//! The Telegram session in SQLite, next to the library database: the keys of the connections to
//! Telegram's data centers, the access hashes of the chats met, which Telegram asks for to reach
//! a chat by its id, and the signed-in account's name. Whoever has the file is signed in.
//!
//! It also keeps the usernames of the public chats met, after signing out too: a public channel
//! the account never joined is not among its chats, and is found again by its username.
//!
//! Updates are never asked for, so their state is only kept in memory.

use grammers_client::session::types::{
    ChannelKind, ChannelState, DcOption, PeerAuth, PeerId, PeerInfo, PeerKind, UpdateState,
    UpdatesState,
};
use grammers_client::session::{BoxFuture, Session, SessionData};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

/// `PRAGMA user_version` of the schema below.
const VERSION: i64 = 1;

const SCHEMA: &str = "
    CREATE TABLE dc_home (dc_id INTEGER NOT NULL PRIMARY KEY);
    CREATE TABLE dc_option (
        dc_id INTEGER NOT NULL PRIMARY KEY,
        ipv4 TEXT NOT NULL,
        ipv6 TEXT NOT NULL,
        auth_key BLOB
    );
    -- By Bot API style id; the hash is the access hash, the subtype a set of `Subtype` bits.
    CREATE TABLE peer_info (peer_id INTEGER NOT NULL PRIMARY KEY, hash INTEGER, subtype INTEGER);
    -- The signed-in account: one row while signed in.
    CREATE TABLE account (id INTEGER NOT NULL PRIMARY KEY CHECK (id = 0), name TEXT NOT NULL);
    -- Public chats by Bot API style id, with their username.
    CREATE TABLE username (peer_id INTEGER NOT NULL PRIMARY KEY, username TEXT NOT NULL);
";

/// Bits of `peer_info.subtype`, as grammers' own SQLite session writes them.
#[repr(u8)]
enum Subtype {
    UserSelf = 1,
    UserBot = 2,
    Megagroup = 4,
    Broadcast = 8,
    Gigagroup = 12,
}

pub(crate) struct SessionStore {
    db: Mutex<Connection>,
    /// Read before every request, so kept in memory.
    dcs: Mutex<Dcs>,
    updates: Mutex<UpdatesState>,
}

struct Dcs {
    home: i32,
    options: HashMap<i32, DcOption>,
}

#[derive(Debug)]
pub(crate) enum SessionError {
    Sql(rusqlite::Error),
    Invalid(String),
}

impl Display for SessionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::Sql(error) => write!(f, "{error}"),
            SessionError::Invalid(what) => write!(f, "invalid Telegram session: {what}"),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<rusqlite::Error> for SessionError {
    fn from(error: rusqlite::Error) -> Self {
        SessionError::Sql(error)
    }
}

type Result<T> = std::result::Result<T, SessionError>;

impl SessionStore {
    /// Opens (creating if needed) the session at `path`, readable by this user only.
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        restrict(path);
        Self::with_connection(conn)
    }

    /// A session that lasts until it is dropped, for tests.
    #[cfg(test)]
    pub(crate) fn in_memory() -> Result<Self> {
        Self::with_connection(Connection::open_in_memory()?)
    }

    fn with_connection(mut conn: Connection) -> Result<Self> {
        conn.busy_timeout(Duration::from_secs(5))?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > VERSION {
            return Err(SessionError::Invalid(format!(
                "schema {version} is newer than this version of N Music ({VERSION})"
            )));
        }
        if version < VERSION {
            let transaction = conn.transaction()?;
            transaction.execute_batch(SCHEMA)?;
            transaction.pragma_update(None, "user_version", VERSION)?;
            transaction.commit()?;
        }
        let defaults = SessionData::default();
        let home = conn
            .query_row("SELECT dc_id FROM dc_home LIMIT 1", [], |row| row.get(0))
            .optional()?
            .unwrap_or(defaults.home_dc);
        let mut options = defaults.dc_options;
        {
            let mut statement =
                conn.prepare("SELECT dc_id, ipv4, ipv6, auth_key FROM dc_option")?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, i32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<Vec<u8>>>(3)?,
                ))
            })?;
            for row in rows {
                let (id, ipv4, ipv6, auth_key) = row?;
                let invalid = |what: &str| SessionError::Invalid(format!("{what} of DC {id}"));
                let auth_key = match auth_key {
                    Some(key) => Some(<[u8; 256]>::try_from(key).map_err(|_| invalid("key"))?),
                    None => None,
                };
                let option = DcOption {
                    id,
                    ipv4: ipv4.parse().map_err(|_| invalid("IPv4 address"))?,
                    ipv6: ipv6.parse().map_err(|_| invalid("IPv6 address"))?,
                    auth_key,
                };
                options.insert(id, option);
            }
        }
        Ok(Self {
            db: Mutex::new(conn),
            dcs: Mutex::new(Dcs { home, options }),
            updates: Mutex::new(UpdatesState::default()),
        })
    }

    /// The name of the signed-in account; `None` while signed out.
    pub(crate) fn account(&self) -> Result<Option<String>> {
        Ok(self
            .db()
            .query_row("SELECT name FROM account WHERE id = 0", [], |row| {
                row.get(0)
            })
            .optional()?)
    }

    /// The account called `name` signed in.
    pub(crate) fn set_account(&self, name: &str) -> Result<()> {
        self.db().execute(
            "INSERT INTO account (id, name) VALUES (0, ?1) \
             ON CONFLICT (id) DO UPDATE SET name = excluded.name",
            [name],
        )?;
        Ok(())
    }

    /// Remembers that the public chat `chat` goes by `username`.
    pub(crate) fn set_username(&self, chat: i64, username: &str) -> Result<()> {
        self.db().execute(
            "INSERT OR REPLACE INTO username (peer_id, username) VALUES (?1, ?2)",
            params![chat, username],
        )?;
        Ok(())
    }

    /// The username the public chat `chat` went by when last met.
    pub(crate) fn username(&self, chat: i64) -> Result<Option<String>> {
        Ok(self
            .db()
            .query_row(
                "SELECT username FROM username WHERE peer_id = ?1",
                [chat],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Forgets the account, the keys and the access hashes of the chats met; not the usernames
    /// of public chats.
    pub(crate) fn clear(&self) -> Result<()> {
        self.db().execute_batch(
            "DELETE FROM account; DELETE FROM peer_info; DELETE FROM dc_option; \
             DELETE FROM dc_home;",
        )?;
        let defaults = SessionData::default();
        *lock(&self.dcs) = Dcs {
            home: defaults.home_dc,
            options: defaults.dc_options,
        };
        *lock(&self.updates) = UpdatesState::default();
        Ok(())
    }

    fn db(&self) -> MutexGuard<'_, Connection> {
        lock(&self.db)
    }

    fn peer_info(&self, peer: PeerId) -> Result<Option<PeerInfo>> {
        let db = self.db();
        let row = match peer.bot_api_dialog_id() {
            Some(id) => db
                .query_row(
                    "SELECT peer_id, hash, subtype FROM peer_info WHERE peer_id = ?1",
                    [id],
                    read_peer,
                )
                .optional()?,
            // The signed-in user, whatever its id.
            None => db
                .query_row(
                    "SELECT peer_id, hash, subtype FROM peer_info WHERE subtype & ?1 LIMIT 1",
                    [Subtype::UserSelf as i64],
                    read_peer,
                )
                .optional()?,
        };
        let Some((id, hash, subtype)) = row else {
            return Ok(None);
        };
        let auth = hash.map(PeerAuth::from_hash);
        let subtype = subtype.map(|subtype| subtype as u8);
        let has = |bit: Subtype| subtype.map(|subtype| subtype & bit as u8 != 0);
        // The self user is asked for by a special id: the row tells its kind and real id.
        let id = match peer.bot_api_dialog_id() {
            Some(_) => peer,
            None => PeerId::user_unchecked(id),
        };
        Ok(Some(match id.kind() {
            PeerKind::User => PeerInfo::User {
                id: id.bare_id_unchecked(),
                auth,
                bot: has(Subtype::UserBot),
                is_self: has(Subtype::UserSelf),
            },
            PeerKind::Chat => PeerInfo::Chat {
                id: id.bare_id_unchecked(),
            },
            PeerKind::Channel => PeerInfo::Channel {
                id: id.bare_id_unchecked(),
                auth,
                kind: subtype.and_then(|subtype| {
                    if subtype & Subtype::Gigagroup as u8 == Subtype::Gigagroup as u8 {
                        Some(ChannelKind::Gigagroup)
                    } else if subtype & Subtype::Broadcast as u8 != 0 {
                        Some(ChannelKind::Broadcast)
                    } else if subtype & Subtype::Megagroup as u8 != 0 {
                        Some(ChannelKind::Megagroup)
                    } else {
                        None
                    }
                }),
            },
        }))
    }

    fn cache_peer_info(&self, peer: &PeerInfo) -> Result<()> {
        let peer = match self.peer_info(peer.id())? {
            Some(mut known) => {
                known.extend_info(peer);
                known
            }
            None => peer.clone(),
        };
        let subtype = match &peer {
            PeerInfo::User { bot, is_self, .. } => {
                let bot = bot.unwrap_or_default().then_some(Subtype::UserBot as u8);
                let me = is_self
                    .unwrap_or_default()
                    .then_some(Subtype::UserSelf as u8);
                bot.into_iter().chain(me).reduce(|a, b| a | b)
            }
            PeerInfo::Chat { .. } => None,
            PeerInfo::Channel { kind, .. } => kind.map(|kind| match kind {
                ChannelKind::Megagroup => Subtype::Megagroup as u8,
                ChannelKind::Broadcast => Subtype::Broadcast as u8,
                ChannelKind::Gigagroup => Subtype::Gigagroup as u8,
            }),
        };
        let hash = match &peer {
            PeerInfo::Chat { .. } => None,
            _ => peer.auth().map(PeerAuth::hash),
        };
        self.db().execute(
            "INSERT OR REPLACE INTO peer_info (peer_id, hash, subtype) VALUES (?1, ?2, ?3)",
            params![
                peer.id().bot_api_dialog_id_unchecked(),
                hash,
                subtype.map(i64::from)
            ],
        )?;
        Ok(())
    }
}

impl Session for SessionStore {
    type Error = SessionError;

    fn home_dc_id(&self) -> Result<i32> {
        Ok(lock(&self.dcs).home)
    }

    fn set_home_dc_id(&self, dc_id: i32) -> BoxFuture<'_, Result<()>> {
        lock(&self.dcs).home = dc_id;
        Box::pin(async move {
            let db = self.db();
            db.execute("DELETE FROM dc_home", [])?;
            db.execute("INSERT INTO dc_home (dc_id) VALUES (?1)", [dc_id])?;
            Ok(())
        })
    }

    fn dc_option(&self, dc_id: i32) -> Result<Option<DcOption>> {
        Ok(lock(&self.dcs).options.get(&dc_id).cloned())
    }

    fn set_dc_option(&self, dc_option: &DcOption) -> BoxFuture<'_, Result<()>> {
        lock(&self.dcs)
            .options
            .insert(dc_option.id, dc_option.clone());
        let dc_option = dc_option.clone();
        Box::pin(async move {
            self.db().execute(
                "INSERT OR REPLACE INTO dc_option (dc_id, ipv4, ipv6, auth_key) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    dc_option.id,
                    dc_option.ipv4.to_string(),
                    dc_option.ipv6.to_string(),
                    dc_option.auth_key.map(|key| key.to_vec())
                ],
            )?;
            Ok(())
        })
    }

    fn peer(&self, peer: PeerId) -> BoxFuture<'_, Result<Option<PeerInfo>>> {
        Box::pin(async move { self.peer_info(peer) })
    }

    fn cache_peer(&self, peer: &PeerInfo) -> BoxFuture<'_, Result<()>> {
        let peer = peer.clone();
        Box::pin(async move { self.cache_peer_info(&peer) })
    }

    fn updates_state(&self) -> BoxFuture<'_, Result<UpdatesState>> {
        Box::pin(async move { Ok(lock(&self.updates).clone()) })
    }

    fn set_update_state(&self, update: UpdateState) -> BoxFuture<'_, Result<()>> {
        let mut state = lock(&self.updates);
        match update {
            UpdateState::All(all) => *state = all,
            UpdateState::Primary { pts, date, seq } => {
                state.pts = pts;
                state.date = date;
                state.seq = seq;
            }
            UpdateState::Secondary { qts } => state.qts = qts,
            UpdateState::Channel { id, pts } => {
                match state.channels.iter_mut().find(|channel| channel.id == id) {
                    Some(channel) => channel.pts = pts,
                    None => state.channels.push(ChannelState { id, pts }),
                }
            }
        }
        Box::pin(async { Ok(()) })
    }
}

fn read_peer(row: &rusqlite::Row) -> rusqlite::Result<(i64, Option<i64>, Option<i64>)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
}

/// A poisoned lock still holds a usable session: what panicked did not leave it half-written.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Lets only this user read the session at `path`.
#[cfg(unix)]
fn restrict(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Err(error) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)) {
        log::warn!("Could not restrict {}: {error}", path.display());
    }
}

/// Elsewhere, the user's own folders are private already.
#[cfg(not(unix))]
fn restrict(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

    fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn keeps_data_centers_and_peers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telegram.db");
        let session = SessionStore::open(&path).unwrap();
        let default_home = session.home_dc_id().unwrap();
        assert!(session.dc_option(default_home).unwrap().is_some());

        let dc = DcOption {
            id: 9,
            ipv4: SocketAddrV4::new(Ipv4Addr::new(1, 2, 3, 4), 443),
            ipv6: SocketAddrV6::new(Ipv6Addr::LOCALHOST, 443, 0, 0),
            auth_key: Some([7; 256]),
        };
        let me = PeerInfo::User {
            id: 42,
            auth: Some(PeerAuth::from_hash(-5)),
            bot: Some(false),
            is_self: Some(true),
        };
        let channel = PeerInfo::Channel {
            id: 1234567890,
            auth: Some(PeerAuth::from_hash(99)),
            kind: Some(ChannelKind::Broadcast),
        };
        block_on(async {
            session.set_home_dc_id(4).await.unwrap();
            session.set_dc_option(&dc).await.unwrap();
            session.cache_peer(&me).await.unwrap();
            session.cache_peer(&channel).await.unwrap();
        });
        session.set_account("Ada").unwrap();
        drop(session);

        let session = SessionStore::open(&path).unwrap();
        assert_eq!(session.home_dc_id().unwrap(), 4);
        assert_eq!(session.dc_option(9).unwrap(), Some(dc));
        assert_eq!(session.account().unwrap().as_deref(), Some("Ada"));
        block_on(async {
            assert_eq!(
                session.peer(PeerId::self_user()).await.unwrap(),
                Some(me.clone())
            );
            assert_eq!(session.peer(me.id()).await.unwrap(), Some(me));
            let reference = session.peer_ref(channel.id()).await.unwrap().unwrap();
            assert_eq!(reference.auth.hash(), 99);
            assert_eq!(
                session.peer(channel.id()).await.unwrap(),
                Some(channel.clone())
            );
        });

        let chat = channel.id().bot_api_dialog_id_unchecked();
        session.set_username(chat, "music").unwrap();
        session.clear().unwrap();
        assert_eq!(session.username(chat).unwrap().as_deref(), Some("music"));
        assert_eq!(session.account().unwrap(), None);
        assert_eq!(session.home_dc_id().unwrap(), default_home);
        assert_eq!(session.dc_option(9).unwrap(), None);
        block_on(async {
            assert_eq!(session.peer(PeerId::self_user()).await.unwrap(), None);
        });
    }

    #[test]
    fn learning_more_of_a_peer_keeps_what_was_known() {
        let session = SessionStore::in_memory().unwrap();
        let known = PeerInfo::Channel {
            id: 7,
            auth: Some(PeerAuth::from_hash(3)),
            kind: None,
        };
        let more = PeerInfo::Channel {
            id: 7,
            auth: None,
            kind: Some(ChannelKind::Megagroup),
        };
        block_on(async {
            session.cache_peer(&known).await.unwrap();
            session.cache_peer(&more).await.unwrap();
            assert_eq!(
                session.peer(known.id()).await.unwrap(),
                Some(PeerInfo::Channel {
                    id: 7,
                    auth: Some(PeerAuth::from_hash(3)),
                    kind: Some(ChannelKind::Megagroup),
                })
            );
        });
    }

    #[cfg(unix)]
    #[test]
    fn only_its_user_can_read_it() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telegram.db");
        drop(SessionStore::open(&path).unwrap());
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
