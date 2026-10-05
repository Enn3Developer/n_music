//! The running playback task and what the queue does on its events.

use super::{ItemId, LoopStatus, QueuePlayer, SAVE_INTERVAL};
use crate::audio::player::{Failure, PlaybackEvent, PlaybackTask};
use crate::TrackTime;
use n_event_bus::{Ctx, EventWriter, Handle, Job, JobToken, Outbox, Tagged};

pub(super) struct PlaybackJob(pub PlaybackTask);

n_event_bus::job_emits!(PlaybackJob => Tagged<PlaybackEvent>);

impl Job for PlaybackJob {
    fn run(self, tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        let events = writer.clone();
        if let Err(error) = self.0.run(|event| events.emit_tagged(tag, event)) {
            writer.emit_tagged(tag, PlaybackEvent::Failed(error));
        }
    }
}

impl Handle<Tagged<PlaybackEvent>> for QueuePlayer {
    fn handle(&mut self, msg: &Tagged<PlaybackEvent>, ctx: &Ctx, out: &mut Outbox) {
        let Some(event) = self.job.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        match event {
            PlaybackEvent::Started { length, paused } => self.started(*length, *paused, out),
            PlaybackEvent::Position {
                time,
                revision,
                discontinuity,
            } => {
                if self.player.seek_revision() == *revision {
                    self.position(*time, *discontinuity, out);
                    self.count_play(out);
                    if (time.position - self.saved_position).abs() >= SAVE_INTERVAL {
                        self.save_state();
                    }
                }
            }
            PlaybackEvent::Advanced { item, length } => self.advanced(*item, *length, out),
            PlaybackEvent::Paused(paused) => {
                self.set_playing(!paused, out);
                if *paused {
                    self.time = self.current_time();
                    self.save_state();
                }
            }
            PlaybackEvent::Ended => self.advance(false, ctx, out),
            PlaybackEvent::Failed(failure) => self.failed(failure, ctx, out),
        }
    }
}

impl QueuePlayer {
    fn started(&mut self, length: f64, paused: bool, out: &mut Outbox) {
        self.loaded = true;
        self.failures = 0;
        self.time.length = length;
        self.report(out);
        self.set_playing(!paused, out);
    }

    /// The gapless successor became audible.
    fn advanced(&mut self, item: ItemId, length: f64, out: &mut Outbox) {
        self.session.arrive(item);
        self.counted = false;
        self.time = TrackTime {
            position: 0.0,
            length,
        };
        if let Some(current) = self.session.current() {
            log::info!("Continuing playback: {}", current.locator);
        }
        self.announce(out);
        self.report(out);
        self.update_next();
        self.sync(out);
    }

    /// Skips a track that cannot play, until every entry failed in a row; stops otherwise.
    fn failed(&mut self, failure: &Failure, ctx: &Ctx, out: &mut Outbox) {
        log::error!("Playback failed: {failure}");
        self.failures += 1;
        if matches!(failure, Failure::Track(_)) && self.failures < self.session.len() {
            // Repeating a track that cannot play would only fail again.
            let manual = self.loop_status == LoopStatus::File;
            self.advance(manual, ctx, out);
            return;
        }
        self.stop(out);
        let time = self.current_time();
        self.position(time, true, out);
    }
}
