//! Keeps the [`Library`] up to date: runs scans, applies playlist changes and records plays.

use super::catalog::{Library, Playlist, PlaylistItem};
use super::db::LibraryDb;
use super::query::{now, PlaylistId};
use super::LibraryPaths;
use crate::jobs::scan::{ScanEvent, ScanJob};
use crate::messages::{
    AddToPlaylist, CreatePlaylist, DeletePlaylist, PlaylistRejected, PlaylistSummary,
    PlaylistsChanged, RemoveFromPlaylist, RenamePlaylist, ScanFinished, ScanRequested,
    SetPlaylistRule, SetPlaylistSort, TrackMetadataLoaded, TrackPlayed, TracksEnumerated,
};
use crate::settings::{LibrarySettings, Options};
use crate::source::{Locator, Providers};
use n_event_bus::{
    Ctx, Handle, Outbox, Registrar, RunningJob, ShutdownRequested, Subscriber, Tagged,
};
use std::any::Any;
use std::sync::Arc;

pub struct LibraryService {
    library: Library,
    /// `None` when the database could not be opened: playlists and statistics then only
    /// last until exit.
    db: Option<LibraryDb>,
    scan: Option<RunningJob>,
    settings: Options<LibrarySettings>,
    paths: LibraryPaths,
    providers: Arc<Providers>,
}

impl LibraryService {
    /// Loads the playlists and statistics; tracks come with the first scan.
    pub fn new(
        providers: Arc<Providers>,
        settings: Options<LibrarySettings>,
        paths: LibraryPaths,
    ) -> Self {
        let db = paths
            .open_db()
            .inspect_err(|error| log::error!("Could not open the library database: {error}"))
            .ok();
        let service = Self {
            library: Library::default(),
            db,
            scan: None,
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

impl Subscriber for LibraryService {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<ScanRequested>();
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
        if self.scan.is_none() {
            // The first scan of a launch: the UIs learn about the saved playlists too.
            self.publish(out);
        }
        self.scan = Some(ctx.jobs.spawn_stream(ScanJob {
            roots: self.settings.get().libraries.clone(),
            paths: self.paths.clone(),
            check_cache: msg.check_cache,
            providers: self.providers.clone(),
        }));
    }
}

impl Handle<Tagged<ScanEvent>> for LibraryService {
    fn handle(&mut self, msg: &Tagged<ScanEvent>, _ctx: &Ctx, out: &mut Outbox) {
        let Some(event) = self.scan.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        match event {
            ScanEvent::Enumerated(tracks) => {
                self.library.write().set_tracks(tracks.clone());
                out.emit(TracksEnumerated {
                    tracks: tracks.clone(),
                });
            }
            ScanEvent::Loaded { index, track } => {
                self.library.write().update_track(*index, track.clone());
                out.emit(TrackMetadataLoaded {
                    index: *index,
                    track: track.clone(),
                });
            }
            ScanEvent::Finished { complete } => {
                if *complete {
                    self.reload();
                    self.publish(out);
                }
                out.emit(ScanFinished {
                    complete: *complete,
                });
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
        out.shutdown_ready();
    }
}
