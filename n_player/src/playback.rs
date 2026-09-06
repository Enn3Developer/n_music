use crate::messages::{
    LoopStatusChanged, Pause, Play, PlayNext, PlayPrevious, PlayTrack, PlaybackChanged,
    PositionChanged, QueueReplaced, Seek, SetLoopStatus, SetVolume, TogglePause, TrackChanged,
    TrackLoaded, VolumeChanged,
};
use n_audio::music_track::MusicTrack;
use n_audio::queue::{LoopStatus, QueuePlayer};
use n_audio::TrackTime;
use n_event_bus::{
    job_emits, Ctx, EventWriter, Handle, Job, JobToken, Outbox, Registrar, RunningJob, Subscriber,
    Tagged, Tick,
};
use pollster::FutureExt;
use std::any::Any;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct LoadTrackJob {
    pub path: PathBuf,
}

job_emits!(LoadTrackJob => Tagged<TrackLoaded>);

impl Job for LoadTrackJob {
    async fn run(self, tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        let result = tokio::task::spawn_blocking(move || {
            MusicTrack::new(self.path.to_string_lossy().to_string())?.get_format()
        })
        .await;
        let format = match result {
            Ok(Ok(format)) => Some(format),
            Ok(Err(err)) => {
                eprintln!("error loading track: {err}");
                None
            }
            Err(err) => {
                eprintln!("error loading track: {err}");
                None
            }
        };
        writer.emit_tagged(tag, TrackLoaded(Mutex::new(format)));
    }
}

pub struct PlaybackEngine {
    player: QueuePlayer,
    index: usize,
    loop_status: LoopStatus,
    current_time: TrackTime,
    load_job: Option<RunningJob>,
    pending_pause: bool,
    pending_seek: Option<f64>,
    last_playback: bool,
    last_volume: f64,
    last_index: usize,
    last_loop_status: LoopStatus,
    last_position: f64,
}

impl PlaybackEngine {
    pub fn new(player: QueuePlayer) -> Self {
        let volume = player.get_volume() as f64;
        Self {
            player,
            index: usize::MAX - 1,
            loop_status: LoopStatus::default(),
            current_time: TrackTime::default(),
            load_job: None,
            pending_pause: false,
            pending_seek: None,
            last_playback: false,
            last_volume: volume,
            last_index: usize::MAX - 1,
            last_loop_status: LoopStatus::default(),
            last_position: 0.0,
        }
    }

    fn playback(&self) -> bool {
        !self.player.is_paused() && self.player.is_playing()
    }

    fn start_track(&mut self, index: usize, ctx: &Ctx) {
        if self.player.is_empty() {
            return;
        }
        self.index = index % self.player.len();
        self.pending_pause = false;
        self.pending_seek = None;
        self.current_time = TrackTime::default();
        if let Err(err) = self.player.end_current().block_on() {
            eprintln!("error happened: {err}");
        }
        if let Some(path) = self.player.get_path_for_file(self.index).block_on() {
            self.load_job = Some(ctx.jobs.spawn_oneshot(LoadTrackJob { path }));
        }
    }

    fn advance(&mut self, force: bool, ctx: &Ctx) {
        let index = if self.index >= self.player.len() {
            0
        } else if force || self.loop_status == LoopStatus::Playlist {
            self.index.wrapping_add(1)
        } else {
            self.index
        };
        self.start_track(index, ctx);
    }

    fn previous(&mut self, ctx: &Ctx) {
        if self.load_job.is_none() && self.current_time.position > 3.0 {
            if let Err(err) = self.player.seek_to(0, 0.0).block_on() {
                eprintln!("error happened while asking to seek: {err}");
            }
        } else {
            let index = if self.index == 0 || self.index > self.player.len() {
                self.player.len().saturating_sub(1)
            } else {
                self.index - 1
            };
            self.start_track(index, ctx);
        }
    }

    fn diff_and_emit(&mut self, out: &mut Outbox) {
        let playback = self.playback();
        if playback != self.last_playback {
            self.last_playback = playback;
            out.emit(PlaybackChanged(playback));
        }

        let volume = self.player.get_volume() as f64;
        if volume != self.last_volume {
            self.last_volume = volume;
            out.emit(VolumeChanged(volume));
        }

        if self.loop_status != self.last_loop_status {
            self.last_loop_status = self.loop_status.clone();
            out.emit(LoopStatusChanged(self.loop_status.clone()));
        }

        if self.current_time.position != self.last_position {
            self.last_position = self.current_time.position;
            out.emit(PositionChanged(self.current_time));
        }

        if self.index != self.last_index {
            self.last_index = self.index;
            if let Some(name) = self.player.queue().get(self.index).cloned() {
                let path = PathBuf::from(self.player.path()).join(name.as_ref());
                out.emit(TrackChanged {
                    index: self.index,
                    path,
                    name,
                });
            }
        }
    }
}

impl Subscriber for PlaybackEngine {
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
        reg.on::<Tick>();
        LoadTrackJob::subscribe(reg);
    }
}

impl Handle<PlayTrack> for PlaybackEngine {
    fn handle(&mut self, msg: &PlayTrack, ctx: &Ctx, out: &mut Outbox) {
        self.start_track(msg.0, ctx);
        self.diff_and_emit(out);
    }
}

impl Handle<PlayNext> for PlaybackEngine {
    fn handle(&mut self, _msg: &PlayNext, ctx: &Ctx, out: &mut Outbox) {
        self.advance(true, ctx);
        self.diff_and_emit(out);
    }
}

impl Handle<PlayPrevious> for PlaybackEngine {
    fn handle(&mut self, _msg: &PlayPrevious, ctx: &Ctx, out: &mut Outbox) {
        self.previous(ctx);
        self.diff_and_emit(out);
    }
}

impl Handle<TogglePause> for PlaybackEngine {
    fn handle(&mut self, _msg: &TogglePause, ctx: &Ctx, out: &mut Outbox) {
        if self.load_job.is_some() {
            self.pending_pause = !self.pending_pause;
        } else if self.player.is_paused() {
            if let Err(err) = self.player.unpause().block_on() {
                eprintln!("error resuming playback: {err}");
            }
        } else if self.player.is_playing() {
            if let Err(err) = self.player.pause().block_on() {
                eprintln!("error pausing playback: {err}");
            }
        } else {
            self.advance(true, ctx);
        }
        self.diff_and_emit(out);
    }
}

impl Handle<Pause> for PlaybackEngine {
    fn handle(&mut self, _msg: &Pause, _ctx: &Ctx, out: &mut Outbox) {
        if self.load_job.is_some() {
            self.pending_pause = true;
        } else if let Err(err) = self.player.pause().block_on() {
            eprintln!("error pausing playback: {err}");
        }
        self.diff_and_emit(out);
    }
}

impl Handle<Play> for PlaybackEngine {
    fn handle(&mut self, _msg: &Play, ctx: &Ctx, out: &mut Outbox) {
        if self.load_job.is_some() {
            self.pending_pause = false;
        } else if !self.player.is_playing() {
            self.advance(true, ctx);
        } else if let Err(err) = self.player.unpause().block_on() {
            eprintln!("error resuming playback: {err}");
        }
        self.diff_and_emit(out);
    }
}

impl Handle<Seek> for PlaybackEngine {
    fn handle(&mut self, msg: &Seek, _ctx: &Ctx, out: &mut Outbox) {
        let seek = match msg {
            Seek::Absolute(value) => *value,
            Seek::Relative(value) => self.current_time.position + value,
        };
        if self.load_job.is_some() {
            self.pending_seek = Some(seek);
            return;
        }
        if let Err(err) = self
            .player
            .seek_to(seek.trunc() as u64, seek.fract())
            .block_on()
        {
            eprintln!("error happened while asking to seek: {err}");
        }
        self.diff_and_emit(out);
    }
}

impl Handle<SetVolume> for PlaybackEngine {
    fn handle(&mut self, msg: &SetVolume, _ctx: &Ctx, out: &mut Outbox) {
        if let Err(err) = self.player.set_volume(msg.0 as f32).block_on() {
            eprintln!("error setting volume: {err}");
        }
        self.diff_and_emit(out);
    }
}

impl Handle<SetLoopStatus> for PlaybackEngine {
    fn handle(&mut self, msg: &SetLoopStatus, _ctx: &Ctx, out: &mut Outbox) {
        self.loop_status = msg.0.clone();
        self.diff_and_emit(out);
    }
}

impl Handle<QueueReplaced> for PlaybackEngine {
    fn handle(&mut self, msg: &QueueReplaced, _ctx: &Ctx, out: &mut Outbox) {
        self.load_job = None;
        self.pending_pause = false;
        self.pending_seek = None;
        self.player.clear().block_on();
        self.player.set_path(msg.path.clone());
        self.player
            .add_all(msg.names.iter().map(String::from))
            .block_on();
        self.player.shrink_to_fit();
        self.index = usize::MAX - 1;
        self.last_index = self.index;
        self.diff_and_emit(out);
    }
}

impl Handle<Tagged<TrackLoaded>> for PlaybackEngine {
    fn handle(&mut self, msg: &Tagged<TrackLoaded>, _ctx: &Ctx, out: &mut Outbox) {
        let Some(loaded) = self.load_job.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        let format = loaded.0.lock().unwrap().take();
        self.load_job = None;
        if let Some(format) = format {
            self.player.deref_mut().play(format);
            if self.pending_pause {
                if let Err(err) = self.player.pause().block_on() {
                    eprintln!("error pausing playback: {err}");
                }
            }
            if let Some(seek) = self.pending_seek.take() {
                if let Err(err) = self
                    .player
                    .seek_to(seek.trunc() as u64, seek.fract())
                    .block_on()
                {
                    eprintln!("error seeking: {err}");
                }
            }
        }
        self.diff_and_emit(out);
    }
}

impl Handle<Tick> for PlaybackEngine {
    fn handle(&mut self, _msg: &Tick, ctx: &Ctx, out: &mut Outbox) {
        if let Some(time) = self.player.get_time() {
            self.current_time = time;
        }
        if self.player.has_ended() {
            self.advance(false, ctx);
        }
        self.diff_and_emit(out);
    }
}
