//! What the queue does on commands and library changes.

use super::{LoopStatus, QueuePlayer};
use crate::audio::output_devices;
use crate::messages::{
    AppVisibilityChanged, ClearQueued, Enqueue, LibraryRootsChanged, ListOutputDevices,
    LoopStatusChanged, OutputDeviceChanged, OutputDevices, Pause, Play, PlayFrom, PlayNext,
    PlayPrevious, QueueChanged, RemoveQueued, ScanFinished, Seek, SetCrossfade, SetLoopStatus,
    SetOutputDevice, SetReplayGain, SetResume, SetShuffle, SetVolume, ShuffleChanged, TogglePause,
    ToggleRepeat, ToggleShuffle, TrackMetadataLoaded, TracksEnumerated, VolumeChanged,
};
use crate::source::Locator;
use n_event_bus::{Ctx, EventWriter, Handle, Job, JobToken, Outbox, ShutdownRequested};
use std::time::Duration;

impl Handle<PlayFrom> for QueuePlayer {
    fn handle(&mut self, msg: &PlayFrom, ctx: &Ctx, out: &mut Outbox) {
        if !ctx.shutting_down {
            self.play_from(&msg.query, msg.start.as_ref(), ctx, out);
        }
    }
}

impl Handle<Enqueue> for QueuePlayer {
    fn handle(&mut self, msg: &Enqueue, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let tracks: Vec<(Locator, Option<u64>)> = {
            let catalog = self.library.read();
            msg.tracks
                .iter()
                .map(|locator| {
                    let fingerprint = catalog.track(locator).and_then(|track| track.fingerprint);
                    (locator.clone(), fingerprint)
                })
                .collect()
        };
        self.session.enqueue(tracks, msg.next);
        self.update_next();
        self.sync(out);
    }
}

impl Handle<RemoveQueued> for QueuePlayer {
    fn handle(&mut self, msg: &RemoveQueued, _ctx: &Ctx, out: &mut Outbox) {
        self.session.remove_queued(msg.0);
        self.update_next();
        self.sync(out);
    }
}

impl Handle<ClearQueued> for QueuePlayer {
    fn handle(&mut self, _msg: &ClearQueued, _ctx: &Ctx, out: &mut Outbox) {
        self.session.clear_queued();
        self.update_next();
        self.sync(out);
    }
}

impl Handle<PlayNext> for QueuePlayer {
    fn handle(&mut self, _msg: &PlayNext, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        self.advance(true, ctx, out);
    }
}

impl Handle<PlayPrevious> for QueuePlayer {
    fn handle(&mut self, _msg: &PlayPrevious, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        if self.loaded && self.current_time().position > 3.0 {
            self.player.seek_to(0.0);
        } else if let Some(item) = self.session.previous(&self.loop_status) {
            self.play(item, ctx, out);
        }
    }
}

impl Handle<TogglePause> for QueuePlayer {
    fn handle(&mut self, _msg: &TogglePause, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        if self.player.is_playing() {
            if self.player.is_paused() {
                self.player.unpause();
            } else {
                self.player.pause();
            }
        } else {
            self.resume(ctx, out);
        }
    }
}

impl Handle<Pause> for QueuePlayer {
    fn handle(&mut self, _msg: &Pause, _ctx: &Ctx, _out: &mut Outbox) {
        self.player.pause();
    }
}

impl Handle<Play> for QueuePlayer {
    fn handle(&mut self, _msg: &Play, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        if self.player.is_playing() {
            self.player.unpause();
        } else {
            self.resume(ctx, out);
        }
    }
}

impl Handle<Seek> for QueuePlayer {
    fn handle(&mut self, msg: &Seek, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let position = match msg {
            Seek::Tracked { position, request } => {
                self.pending_seek_request = Some(*request);
                *position
            }
            Seek::Absolute(position) => *position,
            Seek::Relative(offset) => self.current_time().position + offset,
            Seek::ToItem { item, position } => {
                self.seek_to_item(*item, *position, ctx, out);
                return;
            }
        };
        if self.player.is_playing() && position.is_finite() {
            let length = self.time.length;
            self.seek_clamped(position, length);
            return;
        }
        // A session reopened from the last launch plays from there: move where.
        if self.restored.is_some() && position.is_finite() {
            let length = self.time.length;
            let position = if length > 0.0 {
                position.clamp(0.0, length)
            } else {
                position.max(0.0)
            };
            self.restored = Some(position);
            self.time.position = position;
            self.save_state();
        }
        let time = self.current_time();
        self.position(time, true, out);
    }
}

impl Handle<OutputDeviceChanged> for QueuePlayer {
    fn handle(&mut self, _: &OutputDeviceChanged, ctx: &Ctx, _: &mut Outbox) {
        if !ctx.shutting_down {
            self.player.reload_output();
        }
    }
}

impl Handle<SetOutputDevice> for QueuePlayer {
    fn handle(&mut self, msg: &SetOutputDevice, ctx: &Ctx, _: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        self.player
            .set_output_device(msg.0.as_ref().map(|device| device.id.clone()));
        self.settings
            .update(|settings| settings.output_device = msg.0.clone());
    }
}

impl Handle<ListOutputDevices> for QueuePlayer {
    fn handle(&mut self, _: &ListOutputDevices, ctx: &Ctx, _: &mut Outbox) {
        if !ctx.shutting_down {
            ctx.jobs.spawn_detached(DeviceListing);
        }
    }
}

/// Lists the output devices off the bus: the sound server may take its time to answer.
struct DeviceListing;

impl Job for DeviceListing {
    fn run(self, _tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        writer.emit(OutputDevices(output_devices()));
    }
}

impl Handle<SetVolume> for QueuePlayer {
    fn handle(&mut self, msg: &SetVolume, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down || !msg.0.is_finite() {
            return;
        }
        let volume = msg.0.clamp(0.0, 1.0);
        if self.player.volume() == volume as f32 {
            return;
        }
        self.player.set_volume(volume as f32);
        self.settings.update(|settings| settings.volume = volume);
        out.emit(VolumeChanged(volume));
    }
}

impl Handle<SetLoopStatus> for QueuePlayer {
    fn handle(&mut self, msg: &SetLoopStatus, ctx: &Ctx, out: &mut Outbox) {
        if !ctx.shutting_down && self.loop_status != msg.0 {
            self.loop_status = msg.0.clone();
            // Rounds belong to the old mode.
            self.session.clear_rounds();
            self.update_next();
            self.settings
                .update(|settings| settings.loop_status = msg.0.clone());
            out.emit(LoopStatusChanged(self.loop_status()));
        }
    }
}

impl Handle<ToggleRepeat> for QueuePlayer {
    fn handle(&mut self, _: &ToggleRepeat, ctx: &Ctx, out: &mut Outbox) {
        let status = match self.loop_status {
            // The interim UIs' repeat button only knows repeat all and repeat one.
            LoopStatus::Off | LoopStatus::Playlist => LoopStatus::File,
            LoopStatus::File => LoopStatus::Playlist,
        };
        self.handle(&SetLoopStatus(status), ctx, out);
    }
}

impl Handle<SetShuffle> for QueuePlayer {
    fn handle(&mut self, msg: &SetShuffle, ctx: &Ctx, out: &mut Outbox) {
        if !ctx.shutting_down && self.shuffle != msg.0 {
            self.shuffle = msg.0;
            self.session.set_shuffle(msg.0);
            self.update_next();
            self.settings.update(|settings| settings.shuffle = msg.0);
            out.emit(ShuffleChanged(msg.0));
            self.sync(out);
        }
    }
}

impl Handle<ToggleShuffle> for QueuePlayer {
    fn handle(&mut self, _: &ToggleShuffle, ctx: &Ctx, out: &mut Outbox) {
        self.handle(&SetShuffle(!self.shuffle), ctx, out);
    }
}

impl Handle<TracksEnumerated> for QueuePlayer {
    fn handle(&mut self, _msg: &TracksEnumerated, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        self.offer_library();
        self.session.take_changed();
        // UIs rebuild their lists from the library: tell them the play order again.
        out.emit(QueueChanged {
            entries: self.session.entries(),
        });
        // A session reopened from the last launch shows its track and where it left off.
        if self.restored.is_some() && !std::mem::replace(&mut self.announced, true) {
            let length = self.session.current().and_then(|current| {
                let library = self.library.read();
                library.track(&current.locator).map(|track| track.length)
            });
            self.time.length = length.unwrap_or_default();
            self.announce(out);
            self.report(out);
        }
    }
}

impl Handle<TrackMetadataLoaded> for QueuePlayer {
    fn handle(&mut self, msg: &TrackMetadataLoaded, ctx: &Ctx, out: &mut Outbox) {
        let current = self.session.current().map(|item| &item.locator);
        if !ctx.shutting_down && current == Some(&msg.track.locator) {
            self.announce(out);
        }
    }
}

impl Handle<SetReplayGain> for QueuePlayer {
    fn handle(&mut self, msg: &SetReplayGain, ctx: &Ctx, _out: &mut Outbox) {
        if !ctx.shutting_down {
            self.player.set_replay_gain(msg.0);
            self.settings
                .update(|settings| settings.replay_gain = msg.0);
        }
    }
}

impl Handle<SetResume> for QueuePlayer {
    fn handle(&mut self, msg: &SetResume, ctx: &Ctx, _out: &mut Outbox) {
        if !ctx.shutting_down {
            self.set_resume(msg.0);
            self.settings.update(|settings| settings.resume = msg.0);
        }
    }
}

impl Handle<SetCrossfade> for QueuePlayer {
    fn handle(&mut self, msg: &SetCrossfade, ctx: &Ctx, _out: &mut Outbox) {
        if !ctx.shutting_down {
            self.player.set_crossfade(super::crossfade_duration(msg.0));
            self.settings
                .update(|settings| settings.crossfade = msg.0.max(0.0));
        }
    }
}

impl Handle<ScanFinished> for QueuePlayer {
    fn handle(&mut self, msg: &ScanFinished, ctx: &Ctx, out: &mut Outbox) {
        if !ctx.shutting_down && msg.complete {
            self.reconcile(out);
        }
    }
}

impl Handle<LibraryRootsChanged> for QueuePlayer {
    fn handle(&mut self, msg: &LibraryRootsChanged, ctx: &Ctx, out: &mut Outbox) {
        let removed = self
            .libraries
            .iter()
            .any(|library| !msg.0.contains(library));
        self.libraries = msg.0.clone();
        // The library took the tracks of the removed folders out already.
        if removed && !ctx.shutting_down {
            self.reconcile(out);
        }
    }
}

impl Handle<AppVisibilityChanged> for QueuePlayer {
    fn handle(&mut self, msg: &AppVisibilityChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.player
            .set_progress_interval(msg.0.then_some(Duration::from_millis(250)));
    }
}

impl Handle<ShutdownRequested> for QueuePlayer {
    fn handle(&mut self, _: &ShutdownRequested, _: &Ctx, out: &mut Outbox) {
        if self.player.is_playing() {
            self.time = self.current_time();
        }
        self.save_state();
        self.stop(out);
        if let Some(store) = &mut self.store {
            store.close();
        }
        out.shutdown_ready();
    }
}
