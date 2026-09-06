use crate::jobs::playback::PlaybackJob;
use crate::messages::{
    AppVisibilityChanged, LoopStatusChanged, Pause, Play, PlayNext, PlayPrevious, PlayTrack,
    PlaybackChanged, PositionChanged, QueueReplaced, Seek, SetLoopStatus, SetVolume, Shutdown,
    TogglePause, TrackChanged, VolumeChanged,
};
use n_audio::player::{PlaybackEvent, PlaybackTask};
use n_audio::queue::QueuePlayer;
use n_audio::TrackTime;
use n_event_bus::{Ctx, Handle, Outbox, Registrar, RunningJob, Subscriber, Tagged};
use std::any::Any;
use std::time::Duration;

pub struct Runner {
    player: QueuePlayer,
    job: Option<RunningJob>,
    loaded: bool,
    playing: bool,
    visible: bool,
    time: TrackTime,
    seek_revision: i32,
    pending_seek_revision: Option<i32>,
}

impl Runner {
    pub fn new(path: String, volume: f64) -> Self {
        let mut player = QueuePlayer::new(path);
        player.set_volume(volume as f32);
        player.set_progress_interval(Some(Duration::from_millis(50)));
        Self {
            player,
            job: None,
            loaded: false,
            playing: false,
            visible: true,
            time: TrackTime::default(),
            seek_revision: 0,
            pending_seek_revision: None,
        }
    }

    fn set_playing(&mut self, playing: bool, out: &mut Outbox) {
        if self.playing != playing {
            self.playing = playing;
            out.emit(PlaybackChanged(playing));
        }
    }

    fn position(&mut self, time: TrackTime, discontinuity: bool, out: &mut Outbox) {
        if let Some(revision) = self.pending_seek_revision.take() {
            self.seek_revision = revision;
        }
        self.time = time;
        out.emit(PositionChanged(time, self.seek_revision, discontinuity));
    }

    fn stop(&mut self, out: &mut Outbox) {
        self.player.end_current();
        self.job = None;
        self.loaded = false;
        self.set_playing(false, out);
    }

    fn start(&mut self, task: std::io::Result<PlaybackTask>, ctx: &Ctx, out: &mut Outbox) {
        let Ok(task) = task else {
            return;
        };
        self.loaded = false;
        self.set_playing(false, out);
        self.position(TrackTime::default(), true, out);
        let index = self.player.index();
        if let (Some(path), Some(name)) = (
            self.player.get_path_for_file(index),
            self.player.current_track_name(),
        ) {
            out.emit(TrackChanged { index, path, name });
        }
        self.job = Some(ctx.jobs.spawn_stream(PlaybackJob(task)));
    }

    fn advance(&mut self, force: bool, ctx: &Ctx, out: &mut Outbox) {
        let task = self.player.prepare_next(force);
        self.start(task, ctx, out);
    }

    fn current_time(&self) -> TrackTime {
        self.player.get_time().unwrap_or(self.time)
    }
}

impl Subscriber for Runner {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<PlayTrack>();
        reg.on::<PlayPrevious>();
        reg.on::<PlayNext>();
        reg.on::<TogglePause>();
        reg.on::<Pause>();
        reg.on::<Play>();
        reg.on::<Seek>();
        reg.on::<SetVolume>();
        reg.on::<SetLoopStatus>();
        reg.on::<QueueReplaced>();
        reg.on::<AppVisibilityChanged>();
        reg.on::<Shutdown>();
        PlaybackJob::subscribe(reg);
    }
}

impl Handle<PlayTrack> for Runner {
    fn handle(&mut self, msg: &PlayTrack, ctx: &Ctx, out: &mut Outbox) {
        let task = self.player.prepare_index(msg.0);
        self.start(task, ctx, out);
    }
}

impl Handle<PlayNext> for Runner {
    fn handle(&mut self, _msg: &PlayNext, ctx: &Ctx, out: &mut Outbox) {
        self.advance(true, ctx, out);
    }
}

impl Handle<PlayPrevious> for Runner {
    fn handle(&mut self, _msg: &PlayPrevious, ctx: &Ctx, out: &mut Outbox) {
        if self.loaded && self.current_time().position > 3.0 {
            self.player.seek_to(0, 0.0);
        } else {
            let task = self.player.prepare_previous();
            self.start(task, ctx, out);
        }
    }
}

impl Handle<TogglePause> for Runner {
    fn handle(&mut self, _msg: &TogglePause, ctx: &Ctx, out: &mut Outbox) {
        if self.player.is_playing() {
            if self.player.is_paused() {
                self.player.unpause();
            } else {
                self.player.pause();
            }
        } else {
            self.advance(true, ctx, out);
        }
    }
}

impl Handle<Pause> for Runner {
    fn handle(&mut self, _msg: &Pause, _ctx: &Ctx, _out: &mut Outbox) {
        self.player.pause();
    }
}

impl Handle<Play> for Runner {
    fn handle(&mut self, _msg: &Play, ctx: &Ctx, out: &mut Outbox) {
        if self.player.is_playing() {
            self.player.unpause();
        } else {
            self.advance(true, ctx, out);
        }
    }
}

impl Handle<Seek> for Runner {
    fn handle(&mut self, msg: &Seek, _ctx: &Ctx, out: &mut Outbox) {
        let position = match msg {
            Seek::FromUi { position, revision } => {
                self.pending_seek_revision = Some(*revision);
                *position
            }
            Seek::Absolute(position) => *position,
            Seek::Relative(offset) => self.current_time().position + offset,
        };
        if self.player.is_playing() && position.is_finite() {
            let position = if self.time.length > 0.0 {
                position.clamp(0.0, self.time.length)
            } else {
                position.max(0.0)
            };
            self.player
                .seek_to(position.trunc() as u64, position.fract());
            return;
        }
        self.position(self.time, true, out);
    }
}

impl Handle<SetVolume> for Runner {
    fn handle(&mut self, msg: &SetVolume, _ctx: &Ctx, out: &mut Outbox) {
        if !msg.0.is_finite() {
            return;
        }
        let volume = msg.0.clamp(0.0, 1.0);
        if self.player.get_volume() == volume as f32 {
            return;
        }
        self.player.set_volume(volume as f32);
        out.emit(VolumeChanged(volume));
    }
}

impl Handle<SetLoopStatus> for Runner {
    fn handle(&mut self, msg: &SetLoopStatus, _ctx: &Ctx, out: &mut Outbox) {
        if self.player.loop_status() != msg.0 {
            self.player.set_loop_status(msg.0.clone());
            out.emit(LoopStatusChanged(self.player.loop_status()));
        }
    }
}

impl Handle<QueueReplaced> for Runner {
    fn handle(&mut self, msg: &QueueReplaced, _ctx: &Ctx, out: &mut Outbox) {
        self.stop(out);
        self.player.clear();
        self.player.set_path(msg.path.clone());
        self.player.add_all(msg.names.clone());
        self.position(TrackTime::default(), true, out);
    }
}

impl Handle<AppVisibilityChanged> for Runner {
    fn handle(&mut self, msg: &AppVisibilityChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.visible = msg.0;
        self.player
            .set_progress_interval(self.visible.then_some(Duration::from_millis(50)));
    }
}

impl Handle<Shutdown> for Runner {
    fn handle(&mut self, _msg: &Shutdown, _ctx: &Ctx, out: &mut Outbox) {
        self.stop(out);
    }
}

impl Handle<Tagged<PlaybackEvent>> for Runner {
    fn handle(&mut self, msg: &Tagged<PlaybackEvent>, ctx: &Ctx, out: &mut Outbox) {
        let Some(event) = self.job.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        match event {
            PlaybackEvent::Started { length, paused } => {
                self.loaded = true;
                self.time.length = *length;
                out.emit(PositionChanged(self.time, self.seek_revision, true));
                self.set_playing(!paused, out);
            }
            PlaybackEvent::Position {
                time,
                revision,
                discontinuity,
            } => {
                if self.player.seek_revision() == *revision {
                    self.position(*time, *discontinuity, out);
                }
            }
            PlaybackEvent::Paused(paused) => self.set_playing(!paused, out),
            PlaybackEvent::Ended => self.advance(false, ctx, out),
            PlaybackEvent::Failed(error) => {
                eprintln!("error playing track: {error}");
                self.stop(out);
                self.position(self.time, true, out);
            }
        }
    }
}
