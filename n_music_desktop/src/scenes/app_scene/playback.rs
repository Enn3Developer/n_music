use super::AppScene;
use n_event_bus::{Ctx, Handle, Outbox};
use n_music_core::messages::{PlaybackChanged, PositionChanged, TrackChanged, VolumeChanged};

impl Handle<PlaybackChanged> for AppScene {
    fn handle(&mut self, msg: &PlaybackChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.dirty = true;
        self.playback = msg.0;
        self.apply_ui();
    }
}

impl Handle<TrackChanged> for AppScene {
    fn handle(&mut self, msg: &TrackChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.dirty = true;
        self.playing_index = msg.index as i32;
        self.apply_ui();
    }
}

impl Handle<VolumeChanged> for AppScene {
    fn handle(&mut self, msg: &VolumeChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.dirty = true;
        self.volume = msg.0;
        self.apply_ui();
    }
}

impl Handle<PositionChanged> for AppScene {
    fn handle(&mut self, msg: &PositionChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.position_dirty = true;
        if self.position.floor() != msg.0.position.floor() {
            self.position_str = msg.0.format_pos();
            self.position_text_dirty = true;
        }
        if self.length != msg.0.length {
            self.length = msg.0.length;
            self.length_dirty = true;
        }
        self.position = msg.0.position;
        self.seek_revision = msg.1;
        self.apply_ui();
    }
}
