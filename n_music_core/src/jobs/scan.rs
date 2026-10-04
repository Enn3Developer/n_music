use crate::library::covers::CoverStore;
use crate::library::db::{LibraryDb, ScannedTrack, StoredTrack};
use crate::library::LibraryPaths;
use crate::messages::{ScanFinished, TrackMetadataLoaded, TracksEnumerated};
use crate::music_track::MusicTrack;
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

/// The tracks of every root, in a stable order, and whether every root could be listed.
fn enumerate_audio_files(
    provider: &dyn StreamProvider,
    roots: &[Locator],
) -> (Vec<TrackEntry>, bool) {
    let mut seen = HashSet::new();
    let mut entries = vec![];
    let mut complete = true;
    for root in roots {
        match provider.list_tracks(root) {
            Ok(listed) => entries.extend(
                listed
                    .into_iter()
                    .filter(|entry| seen.insert(entry.locator.clone())),
            ),
            Err(error) => {
                complete = false;
                log::warn!("Could not enumerate library {root}: {error}");
            }
        }
    }
    entries.sort_by_cached_key(|entry| entry.locator.to_string());
    (entries, complete)
}

pub struct ScanJob {
    pub roots: Vec<Locator>,
    pub paths: LibraryPaths,
    /// `false` reloads every track instead of trusting the database.
    pub check_cache: bool,
    pub providers: Arc<Providers>,
}

job_emits!(ScanJob => Tagged<TracksEnumerated>, Tagged<TrackMetadataLoaded>, Tagged<ScanFinished>);

/// A loaded track on its way to the database; `None` when it could not be read.
type Loaded = (Locator, Option<u64>, Option<Track>);

impl Job for ScanJob {
    fn run(self, tag: u64, writer: EventWriter, token: Option<JobToken>) {
        let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        log::info!("Scanning libraries {tag}: {:?}", self.roots);
        let providers = self.providers;
        let (entries, complete) = enumerate_audio_files(providers.as_ref(), &self.roots);
        let len = entries.len();
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
        let mut stored = match db.as_ref().filter(|_| self.check_cache) {
            Some(db) => db.tracks().unwrap_or_else(|error| {
                log::error!("Could not read the library database: {error}");
                HashMap::new()
            }),
            None => HashMap::new(),
        };

        let seen: HashSet<Locator> = entries.iter().map(|entry| entry.locator.clone()).collect();
        let mut covers_exist = HashMap::new();
        let mut tracks = Vec::with_capacity(len);
        let mut pending = vec![];
        let mut unreadable = 0;
        for (index, entry) in entries.into_iter().enumerate() {
            let hit = entry
                .version
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
                    pending.push((index, entry));
                }
            }
        }
        drop(stored);
        log::debug!(
            "Scan {tag}: {len} tracks, {} to load, {unreadable} known unreadable",
            pending.len()
        );
        writer.emit_tagged(tag, TracksEnumerated { tracks });

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
                let writer = &writer;
                let provider = providers.as_ref();
                let covers = &covers;
                let token = &token;
                std::thread::Builder::new()
                    .name(format!("scan {tag} metadata worker {worker}"))
                    .spawn_scoped(scope, move || loop {
                        if token.as_ref().is_some_and(JobToken::is_cancelled) {
                            break;
                        }
                        let Some((index, entry)) = queue.lock().unwrap().next() else {
                            break;
                        };
                        let track = read_track(provider, covers, &entry.locator).map(Arc::new);
                        if let Some(track) = &track {
                            writer.emit_tagged(
                                tag,
                                TrackMetadataLoaded {
                                    index,
                                    track: track.clone(),
                                },
                            );
                        }
                        if tx.send((entry.locator, entry.version, track)).is_err() {
                            break;
                        }
                    })
                    .expect("Failed to spawn a scan metadata worker");
            }
            drop(tx);
            store_loaded(db.as_mut(), &rx);
        });

        let cancelled = token.as_ref().is_some_and(JobToken::is_cancelled);
        if let Some(db) = db.as_mut().filter(|_| !cancelled && complete) {
            let _lock = CLEANUP_LOCK.lock().unwrap();
            if GENERATION.load(Ordering::SeqCst) == generation {
                clean_up(db, &covers, &seen);
            }
        }
        if cancelled {
            log::debug!("Library scan {tag} cancelled");
        } else {
            log::info!("Library scan {tag} completed: {len} tracks");
        }
        writer.emit_tagged(tag, ScanFinished);
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

/// Forgets tracks that are gone and deletes covers no track uses any more.
fn clean_up(db: &mut LibraryDb, covers: &CoverStore, seen: &HashSet<Locator>) {
    match db.retain(seen) {
        Ok(0) => {}
        Ok(deleted) => log::debug!("Removed {deleted} missing tracks from the library"),
        Err(error) => {
            log::error!("Could not remove missing tracks from the library: {error}");
            return;
        }
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
    let (mut info, cover) = MusicTrack::new(provider, locator)
        .read_info()
        .inspect_err(|error| log::debug!("Could not read metadata for {locator}: {error}"))
        .ok()?;
    info.cover = cover.and_then(|data| covers.store(&data, locator));
    Some(info)
}
