use super::AppScene;
use crate::messages::{PlaybackChanged, PositionChanged, TrackChanged, VolumeChanged};
use n_event_bus::{Ctx, Handle, Outbox};

impl Handle<PlaybackChanged> for AppScene {
    fn handle(&mut self, msg: &PlaybackChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.dirty = true;
        self.playback = msg.0;
    }
}

impl Handle<TrackChanged> for AppScene {
    fn handle(&mut self, msg: &TrackChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.dirty = true;
        self.playing_index = msg.index as i32;
    }
}

impl Handle<VolumeChanged> for AppScene {
    fn handle(&mut self, msg: &VolumeChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.dirty = true;
        self.volume = msg.0;
    }
}

impl Handle<PositionChanged> for AppScene {
    fn handle(&mut self, msg: &PositionChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.dirty = true;
        self.position = msg.0.position;
        self.seek_revision = msg.1;
        self.length = msg.0.length;
        self.position_str = msg.0.format_pos();
    }
}
