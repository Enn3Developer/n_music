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

/// Probes the track's container format off-thread (the one blocking part of
/// starting playback) and reports it back through the bus.
pub struct LoadTrackJob {
    pub path: PathBuf,
}

job_emits!(LoadTrackJob => Tagged<TrackLoaded>);

impl Job for LoadTrackJob {
    async fn run(self, tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        let track = match MusicTrack::new(self.path.to_string_lossy().to_string()) {
            Ok(track) => track,
            Err(err) => {
                eprintln!("error happened: {err}");
                return;
            }
        };
        match tokio::task::spawn_blocking(move || track.get_format()).await {
            Ok(Ok(format)) => {
                writer.emit_tagged(tag, TrackLoaded(Mutex::new(Some(format))));
            }
            Ok(Err(err)) => eprintln!("error happened: {err}"),
            Err(err) => eprintln!("error happened: {err}"),
        }
    }
}

/// The only owner of QueuePlayer. Consumes playback commands, anchors state
/// diffing on Tick and emits the *Changed stream everyone else mirrors.
///
/// Index and loop bookkeeping live here (not in QueuePlayer) because starting
/// a track goes through LoadTrackJob instead of QueuePlayer's async play().
pub struct PlaybackEngine {
    player: QueuePlayer,
    index: usize,
    loop_status: LoopStatus,
    current_time: TrackTime,
    load_job: Option<RunningJob>,
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
        if let Err(err) = self.player.end_current().block_on() {
            eprintln!("error happened: {err}");
        }
        if let Some(path) = self.player.get_path_for_file(self.index).block_on() {
            // Storing the new job drops (aborts) a still-loading previous one,
            // and its tag gates a late TrackLoaded from a superseded load.
            self.load_job = Some(ctx.jobs.spawn_oneshot(LoadTrackJob { path }));
        }
    }

    /// `force` mirrors the old `play_next(true)`: advance even when looping a single file.
    fn advance(&mut self, force: bool, ctx: &Ctx) {
        let index = if force || self.loop_status == LoopStatus::Playlist {
            self.index.wrapping_add(1)
        } else {
            self.index
        };
        self.start_track(index, ctx);
    }

    fn previous(&mut self, ctx: &Ctx) {
        if self.current_time.position > 3.0 {
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

    /// Diff current state against the last emitted copy; the Tick handler calls
    /// this unconditionally, command handlers call it for lower latency.
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
        if self.player.is_paused() {
            self.player.unpause().block_on().unwrap();
        } else {
            self.player.pause().block_on().unwrap();
        }
        if !self.player.is_playing() {
            self.advance(true, ctx);
        }
        self.diff_and_emit(out);
    }
}

impl Handle<Pause> for PlaybackEngine {
    fn handle(&mut self, _msg: &Pause, _ctx: &Ctx, out: &mut Outbox) {
        self.player.pause().block_on().unwrap();
        self.diff_and_emit(out);
    }
}

impl Handle<Play> for PlaybackEngine {
    fn handle(&mut self, _msg: &Play, ctx: &Ctx, out: &mut Outbox) {
        self.player.unpause().block_on().unwrap();
        if !self.player.is_playing() {
            self.advance(true, ctx);
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
        self.player.set_volume(msg.0 as f32).block_on().unwrap();
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
        if let Some(format) = loaded.0.lock().unwrap().take() {
            // Explicit deref: QueuePlayer's own async play() would shadow
            // Player's synchronous play(format).
            self.player.deref_mut().play(format);
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
