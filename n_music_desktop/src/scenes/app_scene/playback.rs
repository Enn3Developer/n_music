use super::{AppScene, Changes};
use crate::ui::format_time;
use n_event_bus::{Ctx, Handle, Outbox};
use n_music_core::messages::{
    LoopStatusChanged, PlaybackChanged, PositionChanged, QueueChanged, TrackChanged, VolumeChanged,
};

impl Handle<LoopStatusChanged> for AppScene {
    fn handle(&mut self, msg: &LoopStatusChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.dirty = true;
        self.loop_status = msg.0.clone();
        self.apply_ui();
    }
}

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
        self.playing = Some(msg.track.locator.clone());
        self.playing_index = self.playing_row();
        self.apply_ui();
    }
}

impl Handle<QueueChanged> for AppScene {
    fn handle(&mut self, msg: &QueueChanged, _ctx: &Ctx, _out: &mut Outbox) {
        // Rows follow the play order; tracks the session does not have go last, in library
        // order. A track queued twice shows once.
        let mut placed = vec![false; self.track_count];
        let mut order = Vec::with_capacity(self.track_count);
        for entry in &msg.entries {
            if let Some(&index) = self.index_of.get(&entry.locator) {
                if !std::mem::replace(&mut placed[index], true) {
                    order.push(index);
                }
            }
        }
        order.extend((0..self.track_count).filter(|&index| !placed[index]));
        self.row_of = vec![0; order.len()];
        for (row, &index) in order.iter().enumerate() {
            self.row_of[index] = row;
        }
        self.playing_index = self.playing_row();
        self.changes.push(Changes::Order(order));
        self.dirty = true;
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
        if self.position.floor() != msg.time.position.floor() {
            self.position_str = format_time(msg.time.position);
            self.position_text_dirty = true;
        }
        if self.length != msg.time.length {
            self.length = msg.time.length;
            self.length_dirty = true;
        }
        self.position = msg.time.position;
        self.seek_revision = msg.seek as i32;
        self.apply_ui();
    }
}
