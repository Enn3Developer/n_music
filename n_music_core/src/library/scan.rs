use super::covers::CoverStore;
use super::db::{LibraryDb, ScannedTrack, StoredTrack};
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

/// The tracks of each library in a stable order, `None` for one that could not be listed. A
/// track two libraries list goes to the first.
fn enumerate_audio_files(
    provider: &dyn StreamProvider,
    libraries: &[(Locator, bool)],
) -> Vec<Option<Vec<TrackEntry>>> {
    let mut seen = HashSet::new();
    libraries
        .iter()
        .map(|(root, _)| match provider.list_tracks(root) {
            Ok(listed) => {
                let mut entries: Vec<TrackEntry> = listed
                    .into_iter()
                    .filter(|entry| seen.insert(entry.locator.clone()))
                    .collect();
                entries.sort_by_cached_key(|entry| entry.locator.to_string());
                Some(entries)
            }
            Err(error) => {
                log::warn!("Could not enumerate library {root}: {error}");
                None
            }
        })
        .collect()
}

pub struct ScanJob {
    /// The libraries to scan, each with whether it trusts the database: `false` reloads every
    /// track instead.
    pub libraries: Vec<(Locator, bool)>,
    /// The tracks they listed when last scanned: a complete scan forgets those it no longer
    /// lists.
    pub known: HashSet<Locator>,
    /// The libraries there are when the scan ends: if it scanned them all, it also forgets
    /// every track none of them lists.
    pub settings: Options<LibrarySettings>,
    pub paths: LibraryPaths,
    pub providers: Arc<Providers>,
}

/// A library and its tracks, `None` when it could not be listed.
pub type Listing = (Locator, Option<Vec<Track>>);

/// What a scan reports, in this order.
pub enum ScanEvent {
    /// Each library's tracks, in the order asked. Those not loaded yet are placeholders;
    /// `pending` of them are still to read.
    Listed {
        libraries: Vec<Listing>,
        pending: usize,
    },
    /// A track's metadata, read from its file.
    Loaded(Track),
    /// The scan is over. `complete` when it ran to the end over every library: tracks that are
    /// gone are then forgotten, and references to moved files point to their new location.
    Finished { complete: bool },
}

job_emits!(ScanJob => Tagged<ScanEvent>);

/// A loaded track on its way to the database; `None` when it could not be read.
type Loaded = (Locator, Option<u64>, Option<Track>);

impl Job for ScanJob {
    fn run(self, tag: u64, writer: EventWriter, token: Option<JobToken>) {
        let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        log::info!("Scanning libraries {tag}: {:?}", self.libraries);
        let listings = enumerate_audio_files(self.providers.as_ref(), &self.libraries);
        let complete = listings.iter().all(Option::is_some);
        let seen: HashSet<Locator> = listings
            .iter()
            .flatten()
            .flatten()
            .map(|entry| entry.locator.clone())
            .collect();
        let len = seen.len();
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
        let trusted = self.libraries.iter().any(|&(_, check_cache)| check_cache);
        let stored = match db.as_ref().filter(|_| trusted) {
            Some(db) => db.tracks().unwrap_or_else(|error| {
                log::error!("Could not read the library database: {error}");
                HashMap::new()
            }),
            None => HashMap::new(),
        };

        let (libraries, pending, unreadable) = match_stored(&self.libraries, listings, stored);
        log::debug!(
            "Scan {tag}: {len} tracks, {} to load, {unreadable} known unreadable",
            pending.len()
        );
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
                let forget = if whole {
                    Forget::Unlisted(&seen)
                } else {
                    Forget::Gone(self.known.difference(&seen).cloned().collect())
                };
                clean_up(db, &covers, forget);
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

/// Takes what the database knows of unchanged files, for the libraries that trust it. Returns
/// the tracks of each library (placeholders for those still to read), the entries to read and
/// how many are known unreadable.
fn match_stored(
    libraries: &[(Locator, bool)],
    listings: Vec<Option<Vec<TrackEntry>>>,
    mut stored: HashMap<Locator, StoredTrack>,
) -> (Vec<Listing>, Vec<TrackEntry>, usize) {
    let mut covers_exist = HashMap::new();
    let mut listed = Vec::with_capacity(libraries.len());
    let mut pending = vec![];
    let mut unreadable = 0;
    for ((root, check_cache), entries) in libraries.iter().zip(listings) {
        let Some(entries) = entries else {
            listed.push((root.clone(), None));
            continue;
        };
        let mut tracks = Vec::with_capacity(entries.len());
        for entry in entries {
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
            match hit {
                Some(StoredTrack {
                    info: Some(info), ..
                }) => tracks.push(Arc::new(info)),
                Some(StoredTrack { info: None, .. }) => {
                    unreadable += 1;
                    tracks.push(Arc::new(TrackInfo::placeholder(entry.locator)));
                }
                None => {
                    tracks.push(Arc::new(TrackInfo::placeholder(entry.locator.clone())));
                    pending.push(entry);
                }
            }
        }
        listed.push((root.clone(), Some(tracks)));
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
            if tx.send((entry.locator, entry.version, track)).is_err() {
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

/// Which stored tracks a complete scan forgets.
enum Forget<'a> {
    /// Every one it did not list: it scanned every library.
    Unlisted(&'a HashSet<Locator>),
    /// These, which its libraries listed when last scanned but not any more.
    Gone(Vec<Locator>),
}

/// Forgets tracks that are gone, follows moved files and deletes covers no track uses any more.
fn clean_up(db: &mut LibraryDb, covers: &CoverStore, forget: Forget) {
    let deleted = match forget {
        Forget::Unlisted(seen) => db.retain(seen),
        Forget::Gone(gone) => db.forget(&gone),
    };
    match deleted {
        Ok(0) => {}
        Ok(deleted) => log::debug!("Removed {deleted} missing tracks from the library"),
        Err(error) => {
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
