//! Receives what the core reports on the bus and keeps the [`hub`] up to date.

use crate::hub::{hub, Changed, PlaylistSummary};
use n_event_bus::{Ctx, Handle, Outbox, Registrar, Subscriber};
use n_music_core::messages::{
    LibraryRootsChanged, LoopStatusChanged, PlaybackChanged, PlaylistRejected, PlaylistsChanged,
    PositionChanged, QueueChanged, ScanFinished, ScanRequested, SetLibraryRoots, ShuffleChanged,
    TrackChanged, TrackMetadataLoaded, TrackPlayed, TracksEnumerated, VolumeChanged,
};
use n_music_core::TrackInfo;
use std::any::Any;
use std::sync::Arc;
use std::time::SystemTime;

/// Registered after the core services, so the library is up to date when it runs.
pub struct Listener;

impl Subscriber for Listener {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<ScanRequested>();
        reg.on::<SetLibraryRoots>();
        reg.on::<LibraryRootsChanged>();
        reg.on::<TracksEnumerated>();
        reg.on::<TrackMetadataLoaded>();
        reg.on::<ScanFinished>();
        reg.on::<TrackPlayed>();
        reg.on::<TrackChanged>();
        reg.on::<PlaybackChanged>();
        reg.on::<PositionChanged>();
        reg.on::<VolumeChanged>();
        reg.on::<ShuffleChanged>();
        reg.on::<LoopStatusChanged>();
        reg.on::<QueueChanged>();
        reg.on::<PlaylistsChanged>();
        reg.on::<PlaylistRejected>();
    }
}

impl Handle<ScanRequested> for Listener {
    fn handle(&mut self, _msg: &ScanRequested, ctx: &Ctx, _out: &mut Outbox) {
        if !ctx.shutting_down {
            scan_started();
        }
    }
}

impl Handle<SetLibraryRoots> for Listener {
    fn handle(&mut self, _msg: &SetLibraryRoots, ctx: &Ctx, _out: &mut Outbox) {
        if !ctx.shutting_down {
            scan_started();
        }
    }
}

/// A scan replaces the last one; what it finds is not known yet.
fn scan_started() {
    hub().update(Changed::SCAN | Changed::PROGRESS, |state| {
        state.scanning = true;
        state.found = 0;
        state.unread = 0;
    });
}

impl Handle<LibraryRootsChanged> for Listener {
    fn handle(&mut self, msg: &LibraryRootsChanged, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::ROOTS, |state| {
            state.roots = Arc::new(msg.0.clone())
        });
    }
}

impl Handle<TracksEnumerated> for Listener {
    fn handle(&mut self, msg: &TracksEnumerated, _ctx: &Ctx, _out: &mut Outbox) {
        // The core does not say which tracks it still reads: they are the placeholders.
        let unread = msg
            .tracks
            .iter()
            .filter(|track| ***track == TrackInfo::placeholder(track.locator.clone()))
            .count();
        hub().update(Changed::TRACKS | Changed::PROGRESS, |state| {
            state.found = msg.tracks.len();
            state.unread = unread;
        });
    }
}

impl Handle<TrackMetadataLoaded> for Listener {
    fn handle(&mut self, _msg: &TrackMetadataLoaded, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::METADATA | Changed::PROGRESS, |state| {
            state.unread = state.unread.saturating_sub(1);
        });
    }
}

impl Handle<ScanFinished> for Listener {
    fn handle(&mut self, msg: &ScanFinished, _ctx: &Ctx, _out: &mut Outbox) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0.0, |since| since.as_secs_f64());
        // A complete scan also reloads the play statistics.
        hub().update(Changed::TRACKS | Changed::STATS | Changed::SCAN, |state| {
            state.scanning = false;
            if msg.complete {
                state.updated = Some(now);
            }
        });
    }
}

impl Handle<TrackPlayed> for Listener {
    fn handle(&mut self, _msg: &TrackPlayed, _ctx: &Ctx, _out: &mut Outbox) {
        hub().notify(Changed::STATS);
    }
}

impl Handle<TrackChanged> for Listener {
    fn handle(&mut self, msg: &TrackChanged, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::CURRENT, |state| {
            state.current_item = Some(msg.item);
            state.current = Some(msg.track.clone());
        });
    }
}

impl Handle<PlaybackChanged> for Listener {
    fn handle(&mut self, msg: &PlaybackChanged, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::PLAYBACK, |state| state.playing = msg.0);
    }
}

impl Handle<PositionChanged> for Listener {
    fn handle(&mut self, msg: &PositionChanged, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::POSITION, |state| {
            state.time = msg.time;
            state.seek = msg.seek;
        });
    }
}

impl Handle<VolumeChanged> for Listener {
    fn handle(&mut self, msg: &VolumeChanged, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::VOLUME, |state| state.volume = msg.0);
    }
}

impl Handle<ShuffleChanged> for Listener {
    fn handle(&mut self, msg: &ShuffleChanged, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::MODES, |state| state.shuffle = msg.0);
    }
}

impl Handle<LoopStatusChanged> for Listener {
    fn handle(&mut self, msg: &LoopStatusChanged, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::MODES, |state| state.loop_status = msg.0.clone());
    }
}

impl Handle<QueueChanged> for Listener {
    fn handle(&mut self, msg: &QueueChanged, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::QUEUE, |state| {
            state.queue = Arc::new(msg.entries.clone());
        });
    }
}

impl Handle<PlaylistsChanged> for Listener {
    fn handle(&mut self, msg: &PlaylistsChanged, _ctx: &Ctx, _out: &mut Outbox) {
        let playlists = msg
            .0
            .iter()
            .map(|playlist| PlaylistSummary {
                id: playlist.id,
                name: playlist.name.clone(),
                smart: playlist.smart,
            })
            .collect();
        hub().update(Changed::PLAYLISTS, |state| {
            state.playlists = Arc::new(playlists);
        });
    }
}

impl Handle<PlaylistRejected> for Listener {
    fn handle(&mut self, msg: &PlaylistRejected, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::REJECTED, |state| state.rejected = msg.0.clone());
    }
}
