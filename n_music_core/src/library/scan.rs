use super::covers::CoverStore;
use super::db::{LibraryDb, LibraryTracks, ScannedTrack, StoredTrack};
use super::reader::read_info;
use super::LibraryPaths;
use crate::settings::{LibrarySettings, Options};
use crate::source::{Locator, Providers, StreamProvider, TrackEntry};
use crate::{Track, TrackInfo};
use n_event_bus::{job_emits, EventWriter, Job, JobToken, Tagged};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Latest scan started; only that one may delete stale tracks and covers. A replaced scan is
/// cancelled but its thread can still be running.
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// Serializes deleting stale tracks and collecting unused covers.
static CLEANUP_LOCK: Mutex<()> = Mutex::new(());

/// Loaded tracks are written at least this often, so a killed app (Android does not ask) keeps
/// most of a long first scan.
const CHECKPOINT_INTERVAL: Duration = Duration::from_secs(2);
const CHECKPOINT_TRACKS: usize = 256;

/// The tracks a library lists. One that cannot be listed has those of its last listing that the
/// stream cache has copies of: they play offline.
struct Found {
    entries: Vec<TrackEntry>,
    reachable: bool,
}

/// The tracks of each library, each once, in a stable order. Two libraries may list the same
/// track, like playlists sharing a file or a folder inside another. One that cannot be listed
/// has its `offline` tracks.
fn enumerate_audio_files(
    providers: &Providers,
    libraries: &[(Locator, bool)],
    offline: impl Fn(&Locator) -> Vec<TrackEntry>,
) -> Vec<Found> {
    libraries
        .iter()
        .map(|(root, _)| {
            let (entries, reachable) = match providers.list_tracks(root) {
                Ok(listed) => (listed, true),
                Err(error) => {
                    log::warn!("Could not enumerate library {root}: {error}");
                    (offline(root), false)
                }
            };
            let mut seen = HashSet::new();
            let mut entries: Vec<TrackEntry> = entries
                .into_iter()
                .filter(|entry| seen.insert(entry.locator.clone()))
                .collect();
            entries.sort_by_cached_key(|entry| entry.locator.to_string());
            Found { entries, reachable }
        })
        .collect()
}

pub struct ScanJob {
    /// The libraries to scan, each with whether it trusts the database: `false` reloads every
    /// track instead.
    pub libraries: Vec<(Locator, bool)>,
    /// Those the library has no tracks of yet: it shows them as they were when last listed
    /// until the scan lists them.
    pub restore: Vec<Locator>,
    /// The libraries there are: the scan links the tracks of those only, and if it scanned them
    /// all, it also forgets every track none of them lists.
    pub settings: Options<LibrarySettings>,
    pub paths: LibraryPaths,
    pub providers: Arc<Providers>,
}

/// A library and its tracks; one that could not be listed has those that play offline.
pub struct Listing {
    pub library: Locator,
    pub tracks: Vec<Track>,
    /// It could be listed.
    pub reachable: bool,
}

/// What a scan reports, in this order.
pub enum ScanEvent {
    /// The tracks the libraries to restore listed when last listed, as the database links them,
    /// in the order asked; those it does not link are left out. Those not loaded are
    /// placeholders.
    Restored(Vec<Listing>),
    /// Each library's tracks, in the order asked, linked in the database: it forgot the tracks
    /// no library lists any more. Those not loaded yet are placeholders; `pending` of them are
    /// still to read.
    Listed {
        libraries: Vec<Listing>,
        pending: usize,
    },
    /// A track's metadata, read from its file.
    Loaded(Track),
    /// The scan is over. `complete` when it ran to the end over every library: references to
    /// moved files then point to their new location.
    Finished { complete: bool },
}

job_emits!(ScanJob => Tagged<ScanEvent>);

/// A loaded track on its way to the database; `None` when it could not be read.
type Loaded = (Locator, Option<u64>, Option<Track>);

impl Job for ScanJob {
    fn run(self, tag: u64, writer: EventWriter, token: Option<JobToken>) {
        let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        log::info!("Scanning libraries {tag}: {:?}", self.libraries);
        let covers = CoverStore::open(&self.paths.covers);
        let mut db = self
            .paths
            .open_db()
            .inspect_err(|error| {
                log::error!(
                    "Could not open the library database {}: {error}",
                    self.paths.database.display()
                )
            })
            .ok();
        let linked = match db.as_ref().filter(|_| !self.restore.is_empty()) {
            Some(db) => db.library_tracks(&self.restore).unwrap_or_else(|error| {
                log::error!("Could not read which tracks the libraries list: {error}");
                HashMap::new()
            }),
            None => HashMap::new(),
        };
        let trusted = self.libraries.iter().any(|&(_, check_cache)| check_cache);
        let mut stored = match db.as_ref().filter(|_| trusted || !linked.is_empty()) {
            Some(db) => db.tracks().unwrap_or_else(|error| {
                log::error!("Could not read the library database: {error}");
                HashMap::new()
            }),
            None => HashMap::new(),
        };
        if !linked.is_empty() {
            let restored = restored(&self.restore, linked, &stored);
            writer.emit_tagged(tag, ScanEvent::Restored(restored));
        }
        if !trusted {
            stored.clear();
        }

        let listings = enumerate_audio_files(&self.providers, &self.libraries, |library| {
            db.as_ref().map_or_else(Vec::new, |db| {
                offline_tracks(db, &self.providers, library, &stored)
            })
        });
        let complete = listings.iter().all(|found| found.reachable);
        let seen: HashSet<Locator> = listings
            .iter()
            .flat_map(|found| &found.entries)
            .map(|entry| entry.locator.clone())
            .collect();
        let len = seen.len();
        let (libraries, pending, unreadable) = match_stored(&self.libraries, listings, stored);
        log::debug!(
            "Scan {tag}: {len} tracks, {} to load, {unreadable} known unreadable",
            pending.len()
        );
        // Whether or not every library could be listed: one that could not keeps its links.
        if let Some(db) = db
            .as_mut()
            .filter(|_| !token.as_ref().is_some_and(JobToken::is_cancelled))
        {
            link(db, &libraries, &self.settings);
        }
        writer.emit_tagged(
            tag,
            ScanEvent::Listed {
                libraries,
                pending: pending.len(),
            },
        );
        let loading = Loading {
            tag,
            writer: &writer,
            provider: self.providers.as_ref(),
            covers: &covers,
            token: token.as_ref(),
        };
        loading.run(pending, db.as_mut());

        let cancelled = token.as_ref().is_some_and(JobToken::is_cancelled);
        let complete = complete && !cancelled;
        if let Some(db) = db.as_mut().filter(|_| complete) {
            let _lock = CLEANUP_LOCK.lock().unwrap();
            if GENERATION.load(Ordering::SeqCst) == generation {
                let whole =
                    self.settings.get().libraries.iter().all(|library| {
                        self.libraries.iter().any(|(scanned, _)| scanned == library)
                    });
                clean_up(db, &covers, whole.then_some(&seen));
            }
        }
        if cancelled {
            log::debug!("Library scan {tag} cancelled");
        } else {
            log::info!("Library scan {tag} completed: {len} tracks");
        }
        writer.emit_tagged(tag, ScanEvent::Finished { complete });
    }
}

/// The tracks `library` listed when last listed, as the database links it, that the stream
/// cache has copies of, each with the version it was read at (as `stored`): they play offline.
fn offline_tracks(
    db: &LibraryDb,
    providers: &Providers,
    library: &Locator,
    stored: &HashMap<Locator, StoredTrack>,
) -> Vec<TrackEntry> {
    // Only a remote library lists streamed tracks, the only ones with copies.
    if !library.is_remote() {
        return vec![];
    }
    let linked = db
        .library_tracks(std::slice::from_ref(library))
        .unwrap_or_else(|error| {
            log::error!("Could not read which tracks {library} lists: {error}");
            HashMap::new()
        });
    linked
        .into_values()
        .flatten()
        .filter(|locator| providers.has_copy(locator))
        .map(|locator| TrackEntry {
            version: stored.get(&locator).map(|track| track.version),
            locator,
        })
        .collect()
}

/// What `libraries` listed when last listed, as the database links it (`linked`) and knows the
/// tracks (`stored`): those not read yet, or unreadable, are placeholders.
fn restored(
    libraries: &[Locator],
    mut linked: HashMap<Locator, Vec<Locator>>,
    stored: &HashMap<Locator, StoredTrack>,
) -> Vec<Listing> {
    libraries
        .iter()
        .filter_map(|library| {
            let mut tracks: Vec<Track> = linked
                .remove(library)?
                .into_iter()
                .map(|locator| {
                    let info = stored.get(&locator).and_then(|track| track.info.clone());
                    Arc::new(info.unwrap_or_else(|| TrackInfo::placeholder(locator)))
                })
                .collect();
            tracks.sort_by_cached_key(|track| track.locator.to_string());
            Some(Listing {
                library: library.clone(),
                tracks,
                reachable: true,
            })
        })
        .collect()
}

/// Takes what the database knows of unchanged files, for the libraries that trust it. Returns
/// the tracks of each library (placeholders for those still to read), the entries to read and
/// how many are known unreadable. A track several libraries list is read once, as the first
/// of them asks.
fn match_stored(
    libraries: &[(Locator, bool)],
    listings: Vec<Found>,
    mut stored: HashMap<Locator, StoredTrack>,
) -> (Vec<Listing>, Vec<TrackEntry>, usize) {
    let mut covers_exist = HashMap::new();
    let mut matched: HashMap<Locator, Track> = HashMap::new();
    let mut listed = Vec::with_capacity(libraries.len());
    let mut pending = vec![];
    let mut unreadable = 0;
    for ((root, check_cache), found) in libraries.iter().zip(listings) {
        let mut tracks = Vec::with_capacity(found.entries.len());
        for entry in found.entries {
            if let Some(track) = matched.get(&entry.locator) {
                tracks.push(track.clone());
                continue;
            }
            let hit = entry
                .version
                .filter(|_| *check_cache)
                .and_then(|version| {
                    stored
                        .remove(&entry.locator)
                        .filter(|track| track.version == version)
                })
                // The OS may have cleaned the cache directory.
                .filter(|track| {
                    track
                        .info
                        .as_ref()
                        .and_then(|info| info.cover.clone())
                        .is_none_or(|cover| {
                            *covers_exist
                                .entry(cover)
                                .or_insert_with_key(|cover: &PathBuf| cover.is_file())
                        })
                });
            let track = match hit {
                Some(StoredTrack {
                    info: Some(info), ..
                }) => Arc::new(info),
                Some(StoredTrack { info: None, .. }) => {
                    unreadable += 1;
                    Arc::new(TrackInfo::placeholder(entry.locator.clone()))
                }
                None => {
                    pending.push(entry.clone());
                    Arc::new(TrackInfo::placeholder(entry.locator.clone()))
                }
            };
            matched.insert(entry.locator, track.clone());
            tracks.push(track);
        }
        listed.push(Listing {
            library: root.clone(),
            tracks,
            reachable: found.reachable,
        });
    }
    (listed, pending, unreadable)
}

/// Reads pending tracks on a few workers, reporting each and storing them as they come.
struct Loading<'a> {
    tag: u64,
    writer: &'a EventWriter,
    provider: &'a Providers,
    covers: &'a CoverStore,
    token: Option<&'a JobToken>,
}

impl Loading<'_> {
    fn run(&self, pending: Vec<TrackEntry>, db: Option<&mut LibraryDb>) {
        let concurrency = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
            .min(4)
            .min(pending.len());
        let queue = Mutex::new(pending.into_iter());
        let (tx, rx) = std::sync::mpsc::channel::<Loaded>();
        std::thread::scope(|scope| {
            for worker in 0..concurrency {
                let tx = tx.clone();
                let queue = &queue;
                std::thread::Builder::new()
                    .name(format!("scan {} metadata worker {worker}", self.tag))
                    .spawn_scoped(scope, move || self.work(queue, tx))
                    .expect("Failed to spawn a scan metadata worker");
            }
            drop(tx);
            store_loaded(db, &rx);
        });
    }

    fn work(
        &self,
        queue: &Mutex<std::vec::IntoIter<TrackEntry>>,
        tx: std::sync::mpsc::Sender<Loaded>,
    ) {
        while !self.token.is_some_and(JobToken::is_cancelled) {
            let Some(entry) = queue.lock().unwrap().next() else {
                break;
            };
            let track = read_track(self.provider, self.covers, &entry.locator).map(Arc::new);
            if let Some(track) = &track {
                self.writer
                    .emit_tagged(self.tag, ScanEvent::Loaded(track.clone()));
            }
            // A remote track that could not be read may be back later: it is not stored as
            // unreadable.
            let remote = entry.locator.is_remote();
            let version = entry.version.filter(|_| track.is_some() || !remote);
            if tx.send((entry.locator, version, track)).is_err() {
                break;
            }
        }
    }
}

/// Writes loaded tracks to the database in batches until every worker is done.
fn store_loaded(mut db: Option<&mut LibraryDb>, rx: &std::sync::mpsc::Receiver<Loaded>) {
    let mut batch: Vec<Loaded> = vec![];
    let mut last_write = Instant::now();
    loop {
        let finished = match rx.recv_timeout(CHECKPOINT_INTERVAL) {
            Ok(loaded) => {
                batch.push(loaded);
                false
            }
            Err(RecvTimeoutError::Timeout) => false,
            Err(RecvTimeoutError::Disconnected) => true,
        };
        let due = batch.len() >= CHECKPOINT_TRACKS || last_write.elapsed() >= CHECKPOINT_INTERVAL;
        if !batch.is_empty() && (due || finished) {
            if let Some(db) = db.as_deref_mut() {
                // Tracks without a version cannot be validated later, so they are not stored.
                let scanned: Vec<ScannedTrack> = batch
                    .iter()
                    .filter_map(|(locator, version, track)| {
                        Some(ScannedTrack {
                            locator,
                            version: (*version)?,
                            info: track.as_deref(),
                        })
                    })
                    .collect();
                if let Err(error) = db.save(&scanned) {
                    log::error!("Could not save scanned tracks: {error}");
                }
            }
            batch.clear();
            last_write = Instant::now();
        }
        if finished {
            break;
        }
    }
}

/// Links each library to the tracks it listed, forgetting those no library lists any more.
fn link(db: &mut LibraryDb, listings: &[Listing], settings: &Options<LibrarySettings>) {
    let listed: Vec<LibraryTracks> = listings
        .iter()
        .map(|listing| LibraryTracks {
            library: &listing.library,
            tracks: listing.tracks.iter().map(|track| &track.locator).collect(),
            reachable: listing.reachable,
        })
        .collect();
    match db.link(&listed, || settings.get().libraries.clone()) {
        Ok(0) => {}
        Ok(forgotten) => log::debug!("Removed {forgotten} tracks no library lists any more"),
        Err(error) => log::error!("Could not save which tracks the libraries list: {error}"),
    }
}

/// Forgets every track it did not list when it listed every library (`seen`, then), follows
/// moved files and deletes covers no track uses any more.
fn clean_up(db: &mut LibraryDb, covers: &CoverStore, seen: Option<&HashSet<Locator>>) {
    match seen.map(|seen| db.retain(seen)) {
        None | Some(Ok(0)) => {}
        Some(Ok(deleted)) => log::debug!("Removed {deleted} missing tracks from the library"),
        Some(Err(error)) => {
            log::error!("Could not remove missing tracks from the library: {error}");
            return;
        }
    }
    if let Err(error) = db.repoint() {
        log::error!("Could not follow moved tracks: {error}");
    }
    match db.cover_names() {
        Ok(used) => covers.retain(&used),
        Err(error) => log::error!("Could not list the covers in use: {error}"),
    }
}

/// Reads a track's metadata and stores its cover.
fn read_track(
    provider: &dyn StreamProvider,
    covers: &CoverStore,
    locator: &Locator,
) -> Option<TrackInfo> {
    let (mut info, cover) = read_info(provider, locator)
        .inspect_err(|error| log::debug!("Could not read metadata for {locator}: {error}"))
        .ok()?;
    info.cover = cover.and_then(|data| covers.store(&data, locator));
    Some(info)
}
