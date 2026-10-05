//! Keeps the [`Library`] up to date: runs scans, applies playlist changes and records plays.

use super::catalog::{Library, Listed, Playlist, PlaylistItem};
use super::db::LibraryDb;
use super::query::{now, PlaylistId};
use super::scan::{Listing, ScanEvent, ScanJob};
use super::LibraryPaths;
use crate::messages::{
    AddToPlaylist, CreatePlaylist, DeletePlaylist, LibraryRootsChanged, PlaylistRejected,
    PlaylistSummary, PlaylistsChanged, RemoveFromPlaylist, RenamePlaylist, ScanFinished,
    ScanProgress, ScanRequested, SetLibraryRoots, SetPlaylistRule, SetPlaylistSort,
    TrackMetadataLoaded, TrackPlayed, TracksEnumerated,
};
use crate::settings::{LibrarySettings, Options};
use crate::source::{Locator, Providers};
use crate::Track;
use n_event_bus::{
    Ctx, Handle, Outbox, Registrar, RunningJob, ShutdownRequested, Subscriber, Tagged,
};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct LibraryService {
    library: Library,
    /// `None` when the database could not be opened: playlists and statistics then only
    /// last until exit.
    db: Option<LibraryDb>,
    scan: Option<Scan>,
    /// Libraries to scan once the running scan finished, each with whether that scan trusts
    /// the cache.
    waiting: Vec<(Locator, bool)>,
    /// A scan was asked for since launch: the UIs know the folders and the saved playlists.
    started: bool,
    settings: Options<LibrarySettings>,
    paths: LibraryPaths,
    providers: Arc<Providers>,
}

struct Scan {
    job: RunningJob,
    /// Each with whether its scan trusts the cache.
    libraries: Vec<(Locator, bool)>,
    /// The tracks it listed, and how many of those it still reads.
    found: usize,
    pending: usize,
}

impl LibraryService {
    /// Loads the playlists and statistics into `library`; tracks come with the first scan.
    pub fn new(
        providers: Arc<Providers>,
        library: Library,
        settings: Options<LibrarySettings>,
        paths: LibraryPaths,
    ) -> Self {
        let db = paths
            .open_db()
            .inspect_err(|error| log::error!("Could not open the library database: {error}"))
            .ok();
        let service = Self {
            library,
            db,
            scan: None,
            waiting: vec![],
            started: false,
            settings,
            paths,
            providers,
        };
        service.reload();
        service
    }

    pub fn library(&self) -> Library {
        self.library.clone()
    }

    /// Reads the playlists and statistics again, e.g. after a scan followed moved files.
    fn reload(&self) {
        let Some(db) = &self.db else {
            return;
        };
        let mut catalog = self.library.write();
        match db.playlists() {
            Ok(playlists) => catalog.set_playlists(playlists),
            Err(error) => log::error!("Could not read the playlists: {error}"),
        }
        match db.play_stats() {
            Ok(stats) => catalog.set_stats(stats),
            Err(error) => log::error!("Could not read the play statistics: {error}"),
        }
    }

    /// Scans `library` after the running scan; a reload asked for stays one.
    fn wait_for(&mut self, library: Locator, check_cache: bool) {
        match self
            .waiting
            .iter_mut()
            .find(|(waiting, _)| *waiting == library)
        {
            Some((_, trusted)) => *trusted &= check_cache,
            None => self.waiting.push((library, check_cache)),
        }
    }

    /// Scans every library waiting, unless a scan runs.
    fn start_next(&mut self, ctx: &Ctx) {
        if ctx.shutting_down || self.scan.is_some() || self.waiting.is_empty() {
            return;
        }
        let libraries = std::mem::take(&mut self.waiting);
        let job = ctx.jobs.spawn_stream(ScanJob {
            libraries: libraries.clone(),
            settings: self.settings.clone(),
            paths: self.paths.clone(),
            providers: self.providers.clone(),
        });
        self.scan = Some(Scan {
            job,
            libraries,
            found: 0,
            pending: 0,
        });
    }

    /// Tells which libraries are scanned or waiting, and how far the running scan got.
    fn report(&self, out: &mut Outbox) {
        let mut libraries: Vec<Locator> = vec![];
        let running = self.scan.iter().flat_map(|scan| &scan.libraries);
        for (library, _) in running.chain(&self.waiting) {
            if !libraries.contains(library) {
                libraries.push(library.clone());
            }
        }
        let (found, pending) = self
            .scan
            .as_ref()
            .map_or((0, 0), |scan| (scan.found, scan.pending));
        out.emit(ScanProgress {
            libraries,
            found,
            pending,
        });
    }

    /// Puts the tracks a scan listed in place of those its libraries listed before; the other
    /// libraries keep theirs. Returns the library's tracks.
    fn merge(&mut self, listed: &[Listing]) -> Vec<Track> {
        let scanned: HashSet<&Locator> = listed.iter().map(|listing| &listing.library).collect();
        let mut catalog = self.library.write();
        let stale = unlisted(catalog.listings(), |library| scanned.contains(library));
        let mut tracks: Vec<Track> = catalog
            .tracks()
            .iter()
            .filter(|track| !stale.contains(&track.locator))
            .cloned()
            .collect();
        let mut present: HashSet<Locator> =
            tracks.iter().map(|track| track.locator.clone()).collect();
        for listing in listed {
            tracks.extend(
                listing
                    .tracks
                    .iter()
                    .filter(|track| present.insert(track.locator.clone()))
                    .cloned(),
            );
            let locators = listing.tracks.iter().map(|track| track.locator.clone());
            if listing.reachable {
                catalog.listings_mut().insert(
                    listing.library.clone(),
                    Listed {
                        tracks: locators.collect(),
                        reachable: true,
                    },
                );
            } else {
                // One that could not be listed keeps what it listed before, for the scan that
                // lists it again to forget what is gone.
                let listed = catalog
                    .listings_mut()
                    .entry(listing.library.clone())
                    .or_default();
                listed.reachable = false;
                listed.tracks.extend(locators);
            }
        }
        tracks.sort_by_cached_key(|track| track.locator.to_string());
        catalog.set_tracks(tracks.clone());
        tracks
    }

    /// Takes the tracks of the `removed` libraries out of the library, but those another one
    /// lists too. Returns the library's tracks when that changed them.
    fn remove(&mut self, removed: &[Locator]) -> Option<Vec<Track>> {
        let mut catalog = self.library.write();
        let stale = unlisted(catalog.listings(), |library| removed.contains(library));
        for library in removed {
            catalog.listings_mut().remove(library);
        }
        if stale.is_empty() {
            return None;
        }
        let tracks: Vec<Track> = catalog
            .tracks()
            .iter()
            .filter(|track| !stale.contains(&track.locator))
            .cloned()
            .collect();
        catalog.set_tracks(tracks.clone());
        Some(tracks)
    }

    fn publish(&self, out: &mut Outbox) {
        let summaries = self
            .library
            .read()
            .playlists()
            .map(|playlist| PlaylistSummary {
                id: playlist.id,
                name: playlist.name.clone(),
                smart: playlist.rule.is_some(),
            })
            .collect();
        out.emit(PlaylistsChanged(summaries));
    }

    /// Runs a database write; a failure is logged, the in-memory change stays.
    fn store(&mut self, what: &str, write: impl FnOnce(&mut LibraryDb) -> super::db::Result<()>) {
        if let Some(db) = &mut self.db {
            if let Err(error) = write(db) {
                log::error!("Could not save {what}: {error}");
            }
        }
    }

    /// Applies `change` to a playlist and saves its name, rule and sort.
    fn edit(&mut self, id: PlaylistId, out: &mut Outbox, change: impl FnOnce(&mut Playlist)) {
        let playlist = {
            let mut catalog = self.library.write();
            let Some(playlist) = catalog.playlist_mut(id) else {
                return;
            };
            change(playlist);
            playlist.modified = now();
            playlist.clone()
        };
        self.store("a playlist", |db| db.update_playlist(&playlist));
        self.publish(out);
    }

    /// The tracks to add, with what identifies them should they move.
    fn items(&self, tracks: &[Locator]) -> Vec<(Locator, PlaylistItem)> {
        let catalog = self.library.read();
        let added = now();
        tracks
            .iter()
            .map(|locator| {
                let fingerprint = catalog.track(locator).and_then(|track| track.fingerprint);
                (locator.clone(), PlaylistItem { added, fingerprint })
            })
            .collect()
    }

    fn add(&mut self, id: PlaylistId, items: Vec<(Locator, PlaylistItem)>) {
        {
            let mut catalog = self.library.write();
            let Some(playlist) = catalog.playlist_mut(id) else {
                return;
            };
            for (locator, item) in &items {
                playlist.items.entry(locator.clone()).or_insert(*item);
            }
            playlist.modified = now();
        }
        self.store("playlist tracks", |db| {
            db.add_to_playlist(id, &items, now())
        });
    }
}

/// The tracks the libraries `gone` picks listed and no other library lists.
fn unlisted(
    listings: &HashMap<Locator, Listed>,
    gone: impl Fn(&Locator) -> bool,
) -> HashSet<Locator> {
    let kept: HashSet<&Locator> = listings
        .iter()
        .filter(|(library, _)| !gone(library))
        .flat_map(|(_, listed)| &listed.tracks)
        .collect();
    listings
        .iter()
        .filter(|(library, _)| gone(library))
        .flat_map(|(_, listed)| &listed.tracks)
        .filter(|track| !kept.contains(track))
        .cloned()
        .collect()
}

impl Subscriber for LibraryService {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<ScanRequested>();
        reg.on::<SetLibraryRoots>();
        reg.on::<CreatePlaylist>();
        reg.on::<RenamePlaylist>();
        reg.on::<DeletePlaylist>();
        reg.on::<SetPlaylistRule>();
        reg.on::<SetPlaylistSort>();
        reg.on::<AddToPlaylist>();
        reg.on::<RemoveFromPlaylist>();
        reg.on::<TrackPlayed>();
        reg.on::<ShutdownRequested>();
        ScanJob::subscribe(reg);
    }
}

impl Handle<ScanRequested> for LibraryService {
    fn handle(&mut self, msg: &ScanRequested, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let libraries = self.settings.get().libraries.clone();
        if !self.started {
            // The first scan of a launch: the UIs learn about the folders and the saved
            // playlists too.
            self.started = true;
            out.emit(LibraryRootsChanged(libraries.clone()));
            self.publish(out);
        }
        match &msg.library {
            None => {
                for library in libraries {
                    self.wait_for(library, msg.check_cache);
                }
            }
            Some(library) if libraries.contains(library) => {
                self.wait_for(library.clone(), msg.check_cache);
            }
            Some(library) => log::warn!("Not scanning {library}: it is not a library"),
        }
        self.start_next(ctx);
        self.report(out);
    }
}

impl Handle<SetLibraryRoots> for LibraryService {
    fn handle(&mut self, msg: &SetLibraryRoots, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let old = self.settings.get().libraries.clone();
        self.settings
            .update(|settings| settings.libraries = msg.0.clone());
        let removed: Vec<Locator> = old
            .iter()
            .filter(|library| !msg.0.contains(library))
            .cloned()
            .collect();
        self.waiting
            .retain(|(library, _)| !removed.contains(library));
        // A scan of a removed library starts over without it.
        let stopped = self.scan.take_if(|scan| {
            scan.libraries
                .iter()
                .any(|(library, _)| removed.contains(library))
        });
        for (library, check_cache) in stopped.into_iter().flat_map(|scan| scan.libraries) {
            if !removed.contains(&library) {
                self.wait_for(library, check_cache);
            }
        }
        let tracks = self.remove(&removed);
        if !removed.is_empty() {
            // The database forgets the tracks no other library lists too.
            self.store("a library removal", |db| db.link(&[], &msg.0).map(|_| ()));
        }
        out.emit(LibraryRootsChanged(msg.0.clone()));
        if let Some(tracks) = tracks {
            out.emit(TracksEnumerated { tracks });
        }
        for library in &msg.0 {
            if !old.contains(library) {
                self.wait_for(library.clone(), true);
            }
        }
        self.start_next(ctx);
        self.report(out);
    }
}

impl Handle<Tagged<ScanEvent>> for LibraryService {
    fn handle(&mut self, msg: &Tagged<ScanEvent>, ctx: &Ctx, out: &mut Outbox) {
        let Some(event) = self.scan.as_ref().and_then(|scan| scan.job.open(msg)) else {
            return;
        };
        match event {
            ScanEvent::Listed { libraries, pending } => {
                let tracks = self.merge(libraries);
                if let Some(scan) = &mut self.scan {
                    // Libraries may list the same track.
                    let found: HashSet<&Locator> = libraries
                        .iter()
                        .flat_map(|listing| &listing.tracks)
                        .map(|track| &track.locator)
                        .collect();
                    scan.found = found.len();
                    scan.pending = *pending;
                }
                out.emit(TracksEnumerated { tracks });
                self.report(out);
            }
            ScanEvent::Loaded(track) => {
                if self.library.write().update_track(track.clone()) {
                    if let Some(scan) = &mut self.scan {
                        scan.pending = scan.pending.saturating_sub(1);
                    }
                    out.emit(TrackMetadataLoaded {
                        track: track.clone(),
                    });
                }
            }
            ScanEvent::Finished { complete } => {
                let libraries = self.scan.take().map_or(vec![], |scan| scan.libraries);
                if *complete {
                    self.reload();
                    self.publish(out);
                }
                out.emit(ScanFinished {
                    libraries: libraries.into_iter().map(|(library, _)| library).collect(),
                    complete: *complete,
                });
                self.start_next(ctx);
                self.report(out);
            }
        }
    }
}

impl Handle<CreatePlaylist> for LibraryService {
    fn handle(&mut self, msg: &CreatePlaylist, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let created = now();
        let id = match &mut self.db {
            Some(db) => {
                match db.create_playlist(&msg.name, msg.rule.as_ref(), &msg.sort, created) {
                    Ok(id) => id,
                    Err(error) => {
                        log::error!("Could not create playlist {:?}: {error}", msg.name);
                        out.emit(PlaylistRejected(error.to_string()));
                        return;
                    }
                }
            }
            // Without a database, ids only have to be unique until exit.
            None => PlaylistId(
                self.library
                    .read()
                    .playlists()
                    .map(|playlist| playlist.id.0)
                    .max()
                    .unwrap_or(0)
                    + 1,
            ),
        };
        self.library.write().insert_playlist(Playlist {
            id,
            name: msg.name.clone(),
            // A new playlist cannot be referred to yet, so its rule cannot loop.
            rule: msg.rule.clone(),
            sort: msg.sort.clone(),
            items: Default::default(),
            created,
            modified: created,
        });
        if !msg.tracks.is_empty() {
            let items = self.items(&msg.tracks);
            self.add(id, items);
        }
        self.publish(out);
    }
}

impl Handle<RenamePlaylist> for LibraryService {
    fn handle(&mut self, msg: &RenamePlaylist, _ctx: &Ctx, out: &mut Outbox) {
        self.edit(msg.id, out, |playlist| playlist.name = msg.name.clone());
    }
}

impl Handle<DeletePlaylist> for LibraryService {
    fn handle(&mut self, msg: &DeletePlaylist, _ctx: &Ctx, out: &mut Outbox) {
        self.library.write().remove_playlist(msg.0);
        self.store("a playlist deletion", |db| db.delete_playlist(msg.0));
        self.publish(out);
    }
}

impl Handle<SetPlaylistRule> for LibraryService {
    fn handle(&mut self, msg: &SetPlaylistRule, _ctx: &Ctx, out: &mut Outbox) {
        let loops = msg
            .rule
            .as_ref()
            .is_some_and(|rule| rule.refers_to(&self.library.read(), msg.id));
        if loops {
            out.emit(PlaylistRejected(String::from(
                "A smart playlist's rule cannot refer to the playlist itself",
            )));
            return;
        }
        self.edit(msg.id, out, |playlist| playlist.rule = msg.rule.clone());
    }
}

impl Handle<SetPlaylistSort> for LibraryService {
    fn handle(&mut self, msg: &SetPlaylistSort, _ctx: &Ctx, out: &mut Outbox) {
        self.edit(msg.id, out, |playlist| playlist.sort = msg.sort.clone());
    }
}

impl Handle<AddToPlaylist> for LibraryService {
    fn handle(&mut self, msg: &AddToPlaylist, _ctx: &Ctx, out: &mut Outbox) {
        let items = self.items(&msg.tracks);
        self.add(msg.id, items);
        self.publish(out);
    }
}

impl Handle<RemoveFromPlaylist> for LibraryService {
    fn handle(&mut self, msg: &RemoveFromPlaylist, _ctx: &Ctx, out: &mut Outbox) {
        {
            let mut catalog = self.library.write();
            let Some(playlist) = catalog.playlist_mut(msg.id) else {
                return;
            };
            for locator in &msg.tracks {
                playlist.items.remove(locator);
            }
            playlist.modified = now();
        }
        self.store("playlist tracks", |db| {
            db.remove_from_playlist(msg.id, &msg.tracks, now())
        });
        self.publish(out);
    }
}

impl Handle<TrackPlayed> for LibraryService {
    fn handle(&mut self, msg: &TrackPlayed, _ctx: &Ctx, _out: &mut Outbox) {
        let now = now();
        self.library.write().record_play(msg.locator.clone(), now);
        self.store("a play", |db| {
            db.record_play(&msg.locator, msg.fingerprint, now)
        });
    }
}

impl Handle<ShutdownRequested> for LibraryService {
    fn handle(&mut self, _: &ShutdownRequested, _: &Ctx, out: &mut Outbox) {
        self.scan = None;
        self.waiting.clear();
        out.shutdown_ready();
    }
}
