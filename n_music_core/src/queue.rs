use crate::messages::{
    AppVisibilityChanged, LoopStatusChanged, OutputDeviceChanged, Pause, Play, PlayNext,
    PlayPrevious, PlayTrack, PlaybackChanged, PositionChanged, QueueReplaced, Seek, SetLoopStatus,
    SetVolume, TogglePause, ToggleRepeat, TrackChanged, VolumeChanged,
};
use crate::player::{PlaybackEvent, PlaybackTask, Player};
use crate::settings::{Options, PlaybackSettings};
use crate::source::{LocalProvider, Locator, Providers};
use crate::TrackTime;
use n_event_bus::{
    Ctx, EventWriter, Handle, Job, JobToken, Outbox, Registrar, RunningJob, ShutdownRequested,
    Subscriber, Tagged,
};
use rand::prelude::SliceRandom;
use rand::rng;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cmp::PartialEq;
use std::io;
use std::io::ErrorKind;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;

pub struct PlaybackJob(pub PlaybackTask);

n_event_bus::job_emits!(PlaybackJob => Tagged<PlaybackEvent>);

impl Job for PlaybackJob {
    fn run(self, tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        let events = writer.clone();
        if let Err(error) = self.0.run(|event| events.emit_tagged(tag, event)) {
            writer.emit_tagged(tag, PlaybackEvent::Failed(error.to_string()));
        }
    }
}

#[derive(Default, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum LoopStatus {
    #[default]
    Playlist,
    File,
}

pub struct QueuePlayer {
    queue: Vec<Locator>,
    providers: Arc<Providers>,
    settings: Options<PlaybackSettings>,
    player: Player,
    index: usize,
    loop_status: LoopStatus,
    job: Option<RunningJob>,
    loaded: bool,
    playing: bool,
    time: TrackTime,
    seek_revision: i32,
    pending_seek_revision: Option<i32>,
}

impl Default for QueuePlayer {
    fn default() -> Self {
        Self::new(
            Arc::new(Providers::default().with_local(LocalProvider)),
            Options::in_memory(),
        )
    }
}

impl QueuePlayer {
    /// Starts with the saved volume and loop status, and saves their changes.
    pub fn new(providers: Arc<Providers>, settings: Options<PlaybackSettings>) -> Self {
        let (volume, loop_status) = {
            let saved = settings.get();
            (saved.volume.clamp(0.0, 1.0), saved.loop_status.clone())
        };
        let mut player = Player::new(volume as f32, 1.0);
        player.set_progress_interval(Some(Duration::from_millis(250)));

        QueuePlayer {
            queue: vec![],
            player,
            index: usize::MAX - 1,
            providers,
            settings,
            loop_status,
            job: None,
            loaded: false,
            playing: false,
            time: TrackTime::default(),
            seek_revision: 0,
            pending_seek_revision: None,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn set_loop_status(&mut self, loop_status: LoopStatus) {
        self.loop_status = loop_status;
    }

    pub fn loop_status(&self) -> LoopStatus {
        self.loop_status.clone()
    }

    pub fn locator(&self, i: usize) -> Option<&Locator> {
        self.queue.get(i)
    }

    pub fn queue(&self) -> &[Locator] {
        &self.queue
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.queue.shrink_to_fit();
    }

    #[inline]
    pub fn add(&mut self, locator: Locator) {
        self.queue.push(locator);
    }

    pub fn add_all(&mut self, locators: impl IntoIterator<Item = Locator>) {
        self.queue.extend(locators);
    }

    #[inline]
    pub fn remove(&mut self, index: usize) {
        self.queue.remove(index);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.player.end_current();
        self.queue.clear();
        self.index = usize::MAX - 1;
    }

    #[inline]
    pub fn shuffle(&mut self) {
        self.queue.shuffle(&mut rng());
    }

    pub fn current_locator(&self) -> Option<&Locator> {
        self.queue.get(self.index)
    }

    pub fn prepare_index(&mut self, index: usize) -> io::Result<PlaybackTask> {
        if self.queue.is_empty() {
            return Err(ErrorKind::NotFound.into());
        }
        self.index = index % self.len();
        let locator = self.locator(self.index).ok_or(ErrorKind::NotFound)?.clone();
        Ok(self.player.prepare_track(self.providers.clone(), locator))
    }

    pub fn prepare_next(&mut self, ignore_loop: bool) -> io::Result<PlaybackTask> {
        let index = if self.index >= self.len() {
            0
        } else if ignore_loop || self.loop_status == LoopStatus::Playlist {
            self.index + 1
        } else {
            self.index
        };
        self.prepare_index(index)
    }

    pub fn prepare_previous(&mut self) -> io::Result<PlaybackTask> {
        let index = if self.index == 0 || self.index >= self.len() {
            self.len().saturating_sub(1)
        } else {
            self.index - 1
        };
        self.prepare_index(index)
    }

    pub fn index_of(&self, locator: &Locator) -> Option<usize> {
        self.queue.iter().position(|t| t == locator)
    }
}

impl QueuePlayer {
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
        self.end_current();
        self.job = None;
        self.loaded = false;
        self.set_playing(false, out);
    }

    fn start(&mut self, task: io::Result<PlaybackTask>, ctx: &Ctx, out: &mut Outbox) {
        let Ok(task) = task else {
            return;
        };
        self.loaded = false;
        self.set_playing(false, out);
        self.position(TrackTime::default(), true, out);
        let index = self.index();
        if let Some(locator) = self.current_locator() {
            log::info!("Starting playback: {locator}");
            out.emit(TrackChanged {
                index,
                locator: locator.clone(),
            });
        }
        self.job = Some(ctx.jobs.spawn_stream(PlaybackJob(task)));
    }

    fn advance(&mut self, force: bool, ctx: &Ctx, out: &mut Outbox) {
        let task = self.prepare_next(force);
        self.start(task, ctx, out);
    }

    fn current_time(&self) -> TrackTime {
        self.get_time().unwrap_or(self.time)
    }

    fn seek_clamped(&mut self, position: f64, length: f64) {
        let position = if length > 0.0 {
            position.clamp(0.0, length)
        } else {
            position.max(0.0)
        };
        self.seek_to(position.trunc() as u64, position.fract());
    }

    fn seek_to_track(&mut self, index: usize, position: f64, ctx: &Ctx, out: &mut Outbox) {
        if self.queue.is_empty() || !position.is_finite() {
            return;
        }
        let index = index % self.len();
        if index == self.index && self.is_playing() {
            let length = self.time.length;
            self.seek_clamped(position, length);
            return;
        }
        // Capture the play/pause intent from the existing control before it is replaced; the
        // playback-notification mirror can lag immediate Play/Pause control mutations.
        let paused = !self.is_playing() || self.is_paused();
        let task = self.prepare_index(index);
        if paused {
            self.pause();
        }
        // Queue the seek before starting the worker so it cannot output frames at zero first.
        // The new track's length is unknown yet, so only reject negative positions.
        self.seek_clamped(position, 0.0);
        self.start(task, ctx, out);
    }
}

impl Subscriber for QueuePlayer {
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
        reg.on::<OutputDeviceChanged>();
        reg.on::<Seek>();
        reg.on::<SetVolume>();
        reg.on::<SetLoopStatus>();
        reg.on::<ToggleRepeat>();
        reg.on::<QueueReplaced>();
        reg.on::<AppVisibilityChanged>();
        reg.on::<ShutdownRequested>();
        PlaybackJob::subscribe(reg);
    }
}

impl Handle<PlayTrack> for QueuePlayer {
    fn handle(&mut self, msg: &PlayTrack, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let task = self.prepare_index(msg.0);
        self.start(task, ctx, out);
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
            self.seek_to(0, 0.0);
        } else {
            let task = self.prepare_previous();
            self.start(task, ctx, out);
        }
    }
}

impl Handle<TogglePause> for QueuePlayer {
    fn handle(&mut self, _msg: &TogglePause, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        if self.is_playing() {
            if self.is_paused() {
                self.unpause();
            } else {
                self.pause();
            }
        } else {
            self.advance(true, ctx, out);
        }
    }
}

impl Handle<Pause> for QueuePlayer {
    fn handle(&mut self, _msg: &Pause, _ctx: &Ctx, _out: &mut Outbox) {
        self.pause();
    }
}

impl Handle<Play> for QueuePlayer {
    fn handle(&mut self, _msg: &Play, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        if self.is_playing() {
            self.unpause();
        } else {
            self.advance(true, ctx, out);
        }
    }
}

impl Handle<Seek> for QueuePlayer {
    fn handle(&mut self, msg: &Seek, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let position = match msg {
            Seek::FromUi { position, revision } => {
                self.pending_seek_revision = Some(*revision);
                *position
            }
            Seek::Absolute(position) => *position,
            Seek::Relative(offset) => self.current_time().position + offset,
            Seek::ToTrack { index, position } => {
                self.seek_to_track(*index, *position, ctx, out);
                return;
            }
        };
        if self.is_playing() && position.is_finite() {
            let length = self.time.length;
            self.seek_clamped(position, length);
            return;
        }
        let time = self.current_time();
        self.position(time, true, out);
    }
}

impl Handle<OutputDeviceChanged> for QueuePlayer {
    fn handle(&mut self, _: &OutputDeviceChanged, ctx: &Ctx, _: &mut Outbox) {
        if !ctx.shutting_down {
            self.reload_output();
        }
    }
}

impl Handle<SetVolume> for QueuePlayer {
    fn handle(&mut self, msg: &SetVolume, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down || !msg.0.is_finite() {
            return;
        }
        let volume = msg.0.clamp(0.0, 1.0);
        if self.get_volume() == volume as f32 {
            return;
        }
        self.set_volume(volume as f32);
        self.settings.update(|settings| settings.volume = volume);
        out.emit(VolumeChanged(volume));
    }
}

impl Handle<SetLoopStatus> for QueuePlayer {
    fn handle(&mut self, msg: &SetLoopStatus, ctx: &Ctx, out: &mut Outbox) {
        if !ctx.shutting_down && self.loop_status() != msg.0 {
            self.set_loop_status(msg.0.clone());
            self.settings
                .update(|settings| settings.loop_status = msg.0.clone());
            out.emit(LoopStatusChanged(self.loop_status()));
        }
    }
}

impl Handle<ToggleRepeat> for QueuePlayer {
    fn handle(&mut self, _: &ToggleRepeat, ctx: &Ctx, out: &mut Outbox) {
        let status = match self.loop_status {
            LoopStatus::Playlist => LoopStatus::File,
            LoopStatus::File => LoopStatus::Playlist,
        };
        self.handle(&SetLoopStatus(status), ctx, out);
    }
}

impl Handle<QueueReplaced> for QueuePlayer {
    fn handle(&mut self, msg: &QueueReplaced, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        self.stop(out);
        self.clear();
        self.add_all(msg.tracks.iter().cloned());
        self.position(TrackTime::default(), true, out);
    }
}

impl Handle<AppVisibilityChanged> for QueuePlayer {
    fn handle(&mut self, msg: &AppVisibilityChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.set_progress_interval(msg.0.then_some(Duration::from_millis(250)));
    }
}

impl Handle<ShutdownRequested> for QueuePlayer {
    fn handle(&mut self, _: &ShutdownRequested, _: &Ctx, out: &mut Outbox) {
        self.stop(out);
        out.shutdown_ready();
    }
}

impl Handle<Tagged<PlaybackEvent>> for QueuePlayer {
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
                log::error!("Playback failed: {error}");
                self.stop(out);
                let time = self.current_time();
                self.position(time, true, out);
            }
        }
    }
}

impl Deref for QueuePlayer {
    type Target = Player;

    fn deref(&self) -> &Self::Target {
        &self.player
    }
}

impl DerefMut for QueuePlayer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.player
    }
}
