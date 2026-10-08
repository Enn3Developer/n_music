//! Receives what the core reports on the bus and keeps the [`hub`] up to date.

use crate::hub::{hub, Changed, PlaylistSummary, StreamCache, Telegram, TelegramChats};
use n_event_bus::{Ctx, Handle, Outbox, Registrar, Subscriber};
use n_music_core::messages::{
    LibraryRenamed, LibraryRootsChanged, LoopStatusChanged, OutputDevices, PlaybackChanged,
    PlaylistRejected, PlaylistsChanged, PositionChanged, QueueChanged, ScanFinished, ScanProgress,
    ShuffleChanged, StreamCacheChanged, TelegramChatsFound, TelegramStatusChanged, TrackChanged,
    TrackMetadataLoaded, TrackPlayed, TracksEnumerated, VolumeChanged,
};
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
        reg.on::<LibraryRootsChanged>();
        reg.on::<LibraryRenamed>();
        reg.on::<ScanProgress>();
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
        reg.on::<StreamCacheChanged>();
        reg.on::<OutputDevices>();
        reg.on::<TelegramStatusChanged>();
        reg.on::<TelegramChatsFound>();
    }
}

impl Handle<TelegramStatusChanged> for Listener {
    fn handle(&mut self, msg: &TelegramStatusChanged, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::TELEGRAM, |state| {
            state.telegram = Some(Telegram {
                status: msg.status.clone(),
                busy: msg.busy,
                error: msg.error.clone(),
            });
        });
    }
}

impl Handle<TelegramChatsFound> for Listener {
    fn handle(&mut self, msg: &TelegramChatsFound, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::TELEGRAM, |state| {
            state.telegram_chats = Some(Arc::new(TelegramChats {
                query: msg.query.clone(),
                chats: msg.chats.clone(),
                error: msg.error.clone(),
            }));
        });
    }
}

impl Handle<OutputDevices> for Listener {
    fn handle(&mut self, msg: &OutputDevices, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::DEVICES, |state| {
            state.output_devices = Some(Arc::new(msg.0.clone()));
        });
    }
}

impl Handle<LibraryRootsChanged> for Listener {
    fn handle(&mut self, msg: &LibraryRootsChanged, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::ROOTS, |state| {
            state.roots = Some(Arc::new(msg.0.clone()));
        });
    }
}

impl Handle<LibraryRenamed> for Listener {
    fn handle(&mut self, _msg: &LibraryRenamed, _ctx: &Ctx, _out: &mut Outbox) {
        // The name is in the library.
        hub().notify(Changed::ROOTS);
    }
}

impl Handle<StreamCacheChanged> for Listener {
    fn handle(&mut self, msg: &StreamCacheChanged, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::CACHE, |state| {
            state.cache = Some(StreamCache {
                enabled: msg.enabled,
                limit: msg.limit,
                used: msg.used,
            });
        });
    }
}

impl Handle<ScanProgress> for Listener {
    fn handle(&mut self, msg: &ScanProgress, _ctx: &Ctx, _out: &mut Outbox) {
        hub().update(Changed::SCAN | Changed::PROGRESS, |state| {
            state.scanning = !msg.libraries.is_empty();
            state.updating = Arc::new(msg.libraries.clone());
            state.found = msg.found;
            state.unread = msg.pending;
        });
    }
}

impl Handle<TracksEnumerated> for Listener {
    fn handle(&mut self, _msg: &TracksEnumerated, _ctx: &Ctx, _out: &mut Outbox) {
        hub().notify(Changed::TRACKS);
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
            // Only a scan of every source updates the whole library.
            let whole = state
                .roots
                .as_ref()
                .is_some_and(|roots| roots.iter().all(|root| msg.libraries.contains(root)));
            if msg.complete && whole {
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
