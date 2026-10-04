//! Receives what the core reports on the bus and keeps the [`hub`] up to date.

use crate::hub::{hub, Changed};
use n_event_bus::{Ctx, Handle, Outbox, Registrar, Subscriber};
use n_music_core::messages::{
    LoopStatusChanged, PlaybackChanged, PositionChanged, ScanFinished, ScanRequested,
    SetLibraryRoots, ShuffleChanged, TrackChanged, TrackMetadataLoaded, TrackPlayed,
    TracksEnumerated, VolumeChanged,
};
use std::any::Any;

/// Registered after the core services, so the library is up to date when it runs.
pub struct Listener;

impl Subscriber for Listener {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<ScanRequested>();
        reg.on::<SetLibraryRoots>();
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
    }
}

impl Handle<ScanRequested> for Listener {
    fn handle(&mut self, _msg: &ScanRequested, ctx: &Ctx, _out: &mut Outbox) {
        if !ctx.shutting_down {
            hub().update(Changed::SCAN, |state| state.scanning = true);
        }
    }
}

impl Handle<SetLibraryRoots> for Listener {
    fn handle(&mut self, _msg: &SetLibraryRoots, ctx: &Ctx, _out: &mut Outbox) {
        if !ctx.shutting_down {
            hub().update(Changed::SCAN, |state| state.scanning = true);
        }
    }
}

impl Handle<TracksEnumerated> for Listener {
    fn handle(&mut self, _msg: &TracksEnumerated, _ctx: &Ctx, _out: &mut Outbox) {
        hub().notify(Changed::TRACKS);
    }
}

impl Handle<TrackMetadataLoaded> for Listener {
    fn handle(&mut self, _msg: &TrackMetadataLoaded, _ctx: &Ctx, _out: &mut Outbox) {
        hub().notify(Changed::METADATA);
    }
}

impl Handle<ScanFinished> for Listener {
    fn handle(&mut self, _msg: &ScanFinished, _ctx: &Ctx, _out: &mut Outbox) {
        // A complete scan also reloads the play statistics.
        hub().update(Changed::TRACKS | Changed::STATS | Changed::SCAN, |state| {
            state.scanning = false;
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
            state.current = Some(msg.track.clone())
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
