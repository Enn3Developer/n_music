//! Copies of streamed tracks on disk, recorded as they play. A track with a copy plays from it,
//! without the network, and stays in the library while its own cannot be listed. When space
//! runs out, the most played tracks keep theirs.

mod recording;
mod service;

pub(crate) use service::StreamCacheService;

use self::recording::{Recorder, Recording};
use super::{Locator, OpenedStream, StreamProvider, TrackEntry};
use crate::library::catalog::{Catalog, Library};
use crate::library::write_atomic;
use crate::messages::StreamCacheChanged;
use crate::settings::{Options, StreamCacheSettings};
use n_event_bus::EventWriter;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// Where the last listing of each remote library is kept, in the cache's folder.
const LISTS: &str = "lists";
/// Downloaded at a time when completing a recording.
const CHUNK: usize = 64 * 1024;

pub(crate) struct StreamCache {
    /// Copies are named after their track's key; recordings end in `.part`.
    dir: PathBuf,
    settings: Options<StreamCacheSettings>,
    /// Its play counts decide which tracks keep a copy.
    library: Library,
    writer: EventWriter,
    state: Mutex<State>,
    /// Tells apart the files of recordings of one track.
    serial: AtomicU64,
}

#[derive(Default)]
struct State {
    /// The size of each copy, by key.
    copies: HashMap<u64, u64>,
    /// What the copies take, in bytes.
    used: u64,
    /// Recordings being read into, with how many streams read into each, by key: a track
    /// opened again while it records joins its recording.
    recordings: HashMap<u64, (Arc<Recording>, usize)>,
    /// Recordings to complete, in turn, by downloading what playback did not read.
    completing: VecDeque<Arc<Recording>>,
    /// A thread completes them.
    completer: bool,
}

/// How much a track deserves a copy: the more plays, then the later the last, the more.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Rank {
    plays: u32,
    last_played: i64,
}

impl StreamCache {
    /// Takes up the copies in `dir`; with the cache off, deletes them.
    pub(crate) fn open(
        dir: PathBuf,
        settings: Options<StreamCacheSettings>,
        library: Library,
        writer: EventWriter,
    ) -> Self {
        let mut state = State::default();
        if settings.get().enabled {
            state.copies = copies_in(&dir);
            state.used = state.copies.values().sum();
        } else {
            remove_all(&dir);
        }
        Self {
            dir,
            settings,
            library,
            writer,
            state: Mutex::new(state),
            serial: AtomicU64::new(0),
        }
    }

    /// Reports the settings and what the copies take.
    pub(crate) fn report(&self) {
        let used = self.lock().used;
        let settings = self.settings.get();
        self.writer.emit(StreamCacheChanged {
            enabled: settings.enabled,
            limit: settings.limit,
            used,
        });
    }

    /// Takes new settings: off deletes every copy, a lower limit the lowest ranked ones.
    pub(crate) fn configure(&self, enabled: bool, limit: u64) {
        self.settings.update(|settings| {
            settings.enabled = enabled;
            settings.limit = limit;
        });
        if enabled {
            self.shrink();
        } else {
            self.clear();
        }
        self.report();
    }

    /// Opens `locator` to play it: from its copy when there is one; else from `provider`,
    /// recording what is read while the cache is on.
    pub(crate) fn play(
        self: &Arc<Self>,
        locator: &Locator,
        provider: &Arc<dyn StreamProvider>,
    ) -> io::Result<OpenedStream> {
        let key = key(locator);
        if let Some(copy) = self.copy(key, locator) {
            return Ok(copy);
        }
        let stream = provider.open(locator)?;
        if !self.settings.get().enabled {
            return Ok(stream);
        }
        let Some(recording) = self.record(key, locator, provider, stream.source.byte_len()) else {
            return Ok(stream);
        };
        Ok(OpenedStream {
            source: Box::new(Recorder::new(stream.source, recording, self.clone())),
            extension: stream.extension,
        })
    }

    /// Hands over what the backend `opened` of `locator`, for tags read from the backend to be
    /// up to date, deleting a copy whose size differs from it: the track changed. When the
    /// backend could not open it, opens the copy instead.
    pub(crate) fn check(
        &self,
        locator: &Locator,
        opened: io::Result<OpenedStream>,
    ) -> io::Result<OpenedStream> {
        let key = key(locator);
        let stream = match opened {
            Ok(stream) => stream,
            Err(error) => return self.copy(key, locator).ok_or(error),
        };
        let stale = {
            let mut state = self.lock();
            let stale = state
                .copies
                .get(&key)
                .zip(stream.source.byte_len())
                .is_some_and(|(&copy, len)| copy != len);
            if stale {
                self.remove_copies(&mut state, &[key]);
            }
            stale
        };
        if stale {
            log::info!("Deleted the copy of {locator}: the track changed");
            self.report();
        }
        Ok(stream)
    }

    /// The track at `locator` was played: a complete recording of it may become a copy now.
    pub(crate) fn played(&self, locator: &Locator) {
        let recording = self
            .lock()
            .recordings
            .get(&key(locator))
            .map(|(recording, _)| recording.clone());
        if let Some(recording) = recording.filter(|recording| recording.is_complete()) {
            self.try_keep(&recording);
        }
    }

    /// Keeps what remote library `root` lists, for when it cannot be listed.
    pub(crate) fn listed(&self, root: &Locator, entries: &[TrackEntry]) {
        if !self.settings.get().enabled {
            return;
        }
        let entries: Vec<(&Locator, Option<u64>)> = entries
            .iter()
            .map(|entry| (&entry.locator, entry.version))
            .collect();
        let path = self.list_path(root);
        let kept = serde_json::to_vec(&entries)
            .map_err(io::Error::other)
            .and_then(|bytes| {
                fs::create_dir_all(self.dir.join(LISTS))?;
                write_atomic(&path, &bytes)
            });
        if let Err(error) = kept {
            log::warn!("Could not keep the listing of {root}: {error}");
        }
    }

    /// The tracks remote library `root` listed last that have a copy.
    pub(crate) fn offline(&self, root: &Locator) -> Vec<TrackEntry> {
        if !self.settings.get().enabled {
            return vec![];
        }
        let entries: Vec<(Locator, Option<u64>)> = match fs::read(self.list_path(root)) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                log::warn!("Could not read the listing kept of {root}: {error}");
                vec![]
            }),
            Err(_) => vec![],
        };
        let state = self.lock();
        entries
            .into_iter()
            .filter(|(locator, _)| state.copies.contains_key(&key(locator)))
            .map(|(locator, version)| TrackEntry { locator, version })
            .collect()
    }

    /// Deletes the copies of tracks the library does not hold, and the listings of libraries
    /// other than `libraries`. Call it only once each of them could be listed: before, the
    /// library may lack tracks that have a copy.
    pub(crate) fn prune(&self, libraries: &[Locator]) {
        let pruned = {
            let catalog = self.library.read();
            let held: HashSet<u64> = catalog
                .tracks()
                .iter()
                .filter(|track| track.locator.is_remote())
                .map(|track| key(&track.locator))
                .collect();
            let mut state = self.lock();
            let unheld: Vec<u64> = state
                .copies
                .keys()
                .filter(|key| !held.contains(key))
                .copied()
                .collect();
            self.remove_copies(&mut state, &unheld);
            unheld.len()
        };
        let lists: HashSet<PathBuf> = libraries
            .iter()
            .filter(|library| library.is_remote())
            .map(|library| self.list_path(library))
            .collect();
        for entry in fs::read_dir(self.dir.join(LISTS)).into_iter().flatten() {
            let Ok(entry) = entry else {
                continue;
            };
            if !lists.contains(&entry.path()) {
                remove(&entry.path());
            }
        }
        if pruned > 0 {
            log::info!("Deleted the copies of {pruned} tracks no library lists");
            self.report();
        }
    }

    /// The copy of `locator`, opened; one that cannot be is forgotten.
    fn copy(&self, key: u64, locator: &Locator) -> Option<OpenedStream> {
        if !self.lock().copies.contains_key(&key) {
            return None;
        }
        match File::open(self.copy_path(key)) {
            Ok(file) => Some(OpenedStream {
                source: Box::new(file),
                extension: locator.extension(),
            }),
            Err(error) => {
                log::warn!("Could not open the copy of {locator}: {error}");
                let mut state = self.lock();
                if let Some(size) = state.copies.remove(&key) {
                    state.used -= size;
                }
                drop(state);
                self.report();
                None
            }
        }
    }

    /// The recording of `locator` to read into, joined: the one in progress, else a new one.
    fn record(
        &self,
        key: u64,
        locator: &Locator,
        provider: &Arc<dyn StreamProvider>,
        len: Option<u64>,
    ) -> Option<Arc<Recording>> {
        let mut state = self.lock();
        if let Some((recording, readers)) = state.recordings.get_mut(&key) {
            *readers += 1;
            return Some(recording.clone());
        }
        let serial = self.serial.fetch_add(1, Ordering::Relaxed);
        let path = self.dir.join(format!("{key:016x}.{serial}.part"));
        let created = fs::create_dir_all(&self.dir)
            .and_then(|()| Recording::create(key, locator.clone(), provider.clone(), path, len));
        match created {
            Ok(recording) => {
                let recording = Arc::new(recording);
                state.recordings.insert(key, (recording.clone(), 1));
                Some(recording)
            }
            Err(error) => {
                log::warn!("Could not record {locator}: {error}");
                None
            }
        }
    }

    /// A stream of `recording` was dropped; once none reads into it, it is finished.
    fn leave(self: &Arc<Self>, recording: &Arc<Recording>) {
        let last = {
            let mut state = self.lock();
            let Some((_, readers)) = state.recordings.get_mut(&recording.key) else {
                return;
            };
            *readers -= 1;
            *readers == 0 && state.recordings.remove(&recording.key).is_some()
        };
        if last {
            self.finish(recording.clone());
        }
    }

    /// Keeps a recording nobody reads any more if its track deserves a copy: right away when
    /// it is complete, else once what playback skipped is downloaded.
    fn finish(self: &Arc<Self>, recording: Arc<Recording>) {
        if recording.is_kept() {
            return;
        }
        match recording.missing() {
            Some(missing) if missing.is_empty() => {
                if !self.try_keep(&recording) {
                    discard(&recording);
                }
            }
            Some(_) if self.deserves(&recording) => self.complete_later(recording),
            _ => discard(&recording),
        }
    }

    /// Whether the track of `recording` would get a copy, were it complete.
    fn deserves(&self, recording: &Recording) -> bool {
        let catalog = self.library.read();
        let Some(rank) = rank(&catalog, &recording.locator) else {
            return false;
        };
        let state = self.lock();
        let settings = self.settings.get();
        settings.enabled
            && recording
                .len()
                .is_none_or(|len| room(&catalog, &state, len, rank, settings.limit).is_some())
    }

    /// Makes a complete recording a copy if its track deserves one, deleting lower ranked
    /// copies to make room. Returns whether it is one.
    fn try_keep(&self, recording: &Recording) -> bool {
        let kept = {
            let catalog = self.library.read();
            let mut state = self.lock();
            if recording.is_kept() {
                return true;
            }
            let settings = self.settings.get();
            let fits = rank(&catalog, &recording.locator)
                .zip(recording.len())
                .filter(|_| settings.enabled && !state.copies.contains_key(&recording.key))
                .and_then(|(rank, len)| {
                    Some((len, room(&catalog, &state, len, rank, settings.limit)?))
                });
            fits.and_then(|(len, evicted)| {
                recording.close();
                if let Err(error) = fs::rename(&recording.path, self.copy_path(recording.key)) {
                    log::warn!("Could not keep the copy of {}: {error}", recording.locator);
                    recording.abandon();
                    return None;
                }
                recording.keep();
                self.remove_copies(&mut state, &evicted);
                state.copies.insert(recording.key, len);
                state.used += len;
                Some((len, evicted.len()))
            })
        };
        let Some((len, evicted)) = kept else {
            return false;
        };
        log::info!(
            "Kept a copy of {} ({len} bytes), deleting {evicted} lower ranked",
            recording.locator
        );
        self.report();
        true
    }

    /// Completes `recording` on the cache's thread, after those queued before.
    fn complete_later(self: &Arc<Self>, recording: Arc<Recording>) {
        let start = {
            let mut state = self.lock();
            state.completing.push_back(recording);
            !mem::replace(&mut state.completer, true)
        };
        if !start {
            return;
        }
        let cache = self.clone();
        let spawned = std::thread::Builder::new()
            .name(String::from("stream cache"))
            .spawn(move || cache.complete_queued());
        if let Err(error) = spawned {
            log::error!("Could not start completing copies: {error}");
            let mut state = self.lock();
            state.completer = false;
            for recording in state.completing.drain(..) {
                discard(&recording);
            }
        }
    }

    fn complete_queued(&self) {
        loop {
            let recording = {
                let mut state = self.lock();
                let next = state.completing.pop_front();
                state.completer = next.is_some();
                next
            };
            let Some(recording) = recording else {
                return;
            };
            match self.complete(&recording) {
                Ok(()) => {
                    if !self.try_keep(&recording) {
                        discard(&recording);
                    }
                }
                Err(error) => {
                    log::info!(
                        "Could not complete the copy of {}: {error}",
                        recording.locator
                    );
                    discard(&recording);
                }
            }
        }
    }

    /// Downloads what playback did not read of `recording`.
    fn complete(&self, recording: &Recording) -> io::Result<()> {
        let off = || io::Error::other("The stream cache was turned off");
        if !self.settings.get().enabled {
            return Err(off());
        }
        let mut stream = recording.provider.open(&recording.locator)?;
        let mut buf = vec![0; CHUNK];
        for (start, end) in recording.missing().unwrap_or_default() {
            stream.source.seek(SeekFrom::Start(start))?;
            let mut position = start;
            while end.is_none_or(|end| position < end) {
                if !self.settings.get().enabled {
                    return Err(off());
                }
                let wanted = end.map_or(CHUNK, |end| (end - position).min(CHUNK as u64) as usize);
                let count = stream.source.read(&mut buf[..wanted])?;
                if count == 0 {
                    recording.end(position);
                    break;
                }
                recording.write(position, &buf[..count]);
                position += count as u64;
            }
        }
        match recording.missing() {
            Some(missing) if missing.is_empty() => Ok(()),
            _ => Err(io::Error::other("Parts of the track are still missing")),
        }
    }

    /// Deletes the lowest ranked copies until they fit in the limit.
    fn shrink(&self) {
        let catalog = self.library.read();
        let mut state = self.lock();
        let limit = self.settings.get().limit;
        let mut used = state.used;
        let mut evicted = vec![];
        for (_, key, size) in by_rank(&catalog, &state) {
            if used <= limit {
                break;
            }
            used -= size;
            evicted.push(key);
        }
        if !evicted.is_empty() {
            log::info!("Deleting {} copies to fit in {limit} bytes", evicted.len());
        }
        self.remove_copies(&mut state, &evicted);
    }

    /// Deletes every copy, recording and listing.
    fn clear(&self) {
        let mut state = self.lock();
        for (recording, _) in state.recordings.values() {
            recording.abandon();
        }
        for recording in &state.completing {
            recording.abandon();
        }
        state.copies.clear();
        state.used = 0;
        remove_all(&self.dir);
    }

    /// Deletes the copies of `keys`.
    fn remove_copies(&self, state: &mut State, keys: &[u64]) {
        for key in keys {
            if let Some(size) = state.copies.remove(key) {
                state.used -= size;
                remove(&self.copy_path(*key));
            }
        }
    }

    fn copy_path(&self, key: u64) -> PathBuf {
        self.dir.join(format!("{key:016x}"))
    }

    fn list_path(&self, library: &Locator) -> PathBuf {
        self.dir
            .join(LISTS)
            .join(format!("{:016x}.json", key(library)))
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }
}

/// Names the copy of a track, or the listing of a library.
fn key(locator: &Locator) -> u64 {
    // Serialized, its kind is part of it: the text alone could be alike across kinds.
    xxhash_rust::xxh3::xxh3_64(&serde_json::to_vec(locator).unwrap_or_default())
}

/// How much the track at `locator` deserves a copy; `None` when the library does not hold it
/// or it was never played.
fn rank(catalog: &Catalog, locator: &Locator) -> Option<Rank> {
    catalog.track(locator)?;
    let stats = catalog.stats(locator).filter(|stats| stats.plays > 0)?;
    Some(Rank {
        plays: stats.plays,
        last_played: stats.last_played,
    })
}

/// The copies as `(rank, key, size)`, lowest ranked first: those of tracks without a rank
/// before any.
fn by_rank(catalog: &Catalog, state: &State) -> Vec<(Option<Rank>, u64, u64)> {
    let ranks: HashMap<u64, Rank> = catalog
        .tracks()
        .iter()
        .filter(|track| track.locator.is_remote())
        .filter_map(|track| Some((key(&track.locator), rank(catalog, &track.locator)?)))
        .collect();
    let mut copies: Vec<_> = state
        .copies
        .iter()
        .map(|(&key, &size)| (ranks.get(&key).copied(), key, size))
        .collect();
    copies.sort_unstable();
    copies
}

/// The copies to delete, lowest ranked first, for one of `len` bytes ranked `rank` to fit in
/// `limit`; `None` when it would not fit without deleting one ranked as high or higher.
fn room(catalog: &Catalog, state: &State, len: u64, rank: Rank, limit: u64) -> Option<Vec<u64>> {
    if len > limit {
        return None;
    }
    let mut used = state.used;
    if used + len <= limit {
        return Some(vec![]);
    }
    let mut evicted = vec![];
    for (copy, key, size) in by_rank(catalog, state) {
        if copy.is_some_and(|copy| copy >= rank) {
            return None;
        }
        used -= size;
        evicted.push(key);
        if used + len <= limit {
            return Some(evicted);
        }
    }
    None
}

/// The copies in `dir`, by key, with their size. Recordings an earlier launch left are deleted.
fn copies_in(dir: &Path) -> HashMap<u64, u64> {
    let mut copies = HashMap::new();
    for entry in fs::read_dir(dir).into_iter().flatten() {
        let Ok(entry) = entry else {
            continue;
        };
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".part") {
            remove(&entry.path());
            continue;
        }
        let key = Some(&*name)
            .filter(|name| name.len() == 16)
            .and_then(|name| u64::from_str_radix(name, 16).ok());
        let size = entry
            .metadata()
            .ok()
            .filter(|metadata| metadata.is_file())
            .map(|metadata| metadata.len());
        if let Some((key, size)) = key.zip(size) {
            copies.insert(key, size);
        }
    }
    copies
}

/// Deletes a recording that does not become a copy.
fn discard(recording: &Recording) {
    recording.close();
    remove(&recording.path);
}

fn remove(path: &Path) {
    if let Err(error) = fs::remove_file(path) {
        if error.kind() != io::ErrorKind::NotFound {
            log::warn!("Could not delete {}: {error}", path.display());
        }
    }
}

/// Deletes `dir` and all in it.
fn remove_all(dir: &Path) {
    if let Err(error) = fs::remove_dir_all(dir) {
        if error.kind() != io::ErrorKind::NotFound {
            log::warn!("Could not delete {}: {error}", dir.display());
        }
    }
}
