//! The play session on the bus: what plays next and playback control. It lasts until exit.

mod session;

use crate::audio::player::{Failure, Next, PlaybackEvent, PlaybackTask, Player};
use crate::library::catalog::Library;
use crate::library::query::Query;
use crate::messages::{
    AppVisibilityChanged, ClearQueued, Enqueue, LoopStatusChanged, OutputDeviceChanged, Pause,
    Play, PlayFrom, PlayNext, PlayPrevious, PlaybackChanged, PositionChanged, QueueChanged,
    RemoveQueued, ScanFinished, Seek, SetLoopStatus, SetReplayGain, SetShuffle, SetVolume,
    ShuffleChanged, TogglePause, ToggleRepeat, ToggleShuffle, TrackChanged, TrackMetadataLoaded,
    TrackPlayed, TracksEnumerated, VolumeChanged,
};
use crate::settings::{Options, PlaybackSettings};
use crate::source::{Locator, Providers};
use crate::{TrackInfo, TrackTime};
use n_event_bus::{
    Ctx, EventWriter, Handle, Job, JobToken, Outbox, Registrar, RunningJob, ShutdownRequested,
    Subscriber, Tagged,
};
use serde::{Deserialize, Serialize};
use session::Session;
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

pub struct PlaybackJob(pub PlaybackTask);

n_event_bus::job_emits!(PlaybackJob => Tagged<PlaybackEvent>);

impl Job for PlaybackJob {
    fn run(self, tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        let events = writer.clone();
        if let Err(error) = self.0.run(|event| events.emit_tagged(tag, event)) {
            writer.emit_tagged(tag, PlaybackEvent::Failed(error));
        }
    }
}

#[derive(Default, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
/// What happens at the end of the context. Up-next tracks always play once, first.
pub enum LoopStatus {
    /// Stops after the last track; with shuffle, after the round. Skipping on goes on.
    Off,
    /// Starts the context over; with shuffle, each round in a new order.
    #[default]
    Playlist,
    /// Repeats the current track. Skipping still moves on.
    File,
}

/// One entry of the session, unique even when a track is in it twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub struct QueueEntry {
    pub item: ItemId,
    pub locator: Locator,
    /// Queued with [`Enqueue`] rather than taken from the context.
    pub queued: bool,
}

pub struct QueuePlayer {
    session: Session,
    library: Library,
    shuffle: bool,
    providers: Arc<Providers>,
    settings: Options<PlaybackSettings>,
    player: Player,
    loop_status: LoopStatus,
    job: Option<RunningJob>,
    loaded: bool,
    playing: bool,
    time: TrackTime,
    /// The last [`Seek::Tracked`] request that applied, and one waiting for its position.
    seek_request: u64,
    pending_seek_request: Option<u64>,
    /// The context ran out with looping off: playing again starts it over.
    finished: bool,
    /// This play of the current item was counted already.
    counted: bool,
    /// Tracks that failed in a row; once every entry did, playback stops.
    failures: usize,
}

impl QueuePlayer {
    /// Starts with the saved volume and modes, and nothing to play yet.
    pub fn new(
        providers: Arc<Providers>,
        settings: Options<PlaybackSettings>,
        library: Library,
    ) -> Self {
        let (volume, loop_status, shuffle, replay_gain) = {
            let saved = settings.get();
            (
                saved.volume.clamp(0.0, 1.0),
                saved.loop_status.clone(),
                saved.shuffle,
                saved.replay_gain,
            )
        };
        let mut player = Player::new(volume as f32, replay_gain);
        player.set_progress_interval(Some(Duration::from_millis(250)));

        QueuePlayer {
            session: Session::default(),
            library,
            shuffle,
            player,
            providers,
            settings,
            loop_status,
            job: None,
            loaded: false,
            playing: false,
            time: TrackTime::default(),
            seek_request: 0,
            pending_seek_request: None,
            finished: false,
            counted: false,
            failures: 0,
        }
    }

    pub fn volume(&self) -> f64 {
        f64::from(self.player.volume())
    }

    pub fn loop_status(&self) -> LoopStatus {
        self.loop_status.clone()
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    /// Publishes the entries when they changed.
    fn sync(&mut self, out: &mut Outbox) {
        if self.session.take_changed() {
            out.emit(QueueChanged {
                entries: self.session.entries(),
            });
        }
    }

    /// Tells the running task what follows the current item, for gapless playback.
    fn update_next(&mut self) {
        let next = self
            .session
            .current()
            .map(|item| item.id)
            .and_then(|after| {
                let item = self.session.next(&self.loop_status, self.shuffle, false)?;
                Some(Next {
                    after,
                    item,
                    locator: self.session.get(item)?.locator.clone(),
                })
            });
        self.player.set_next(next);
    }

    /// Makes `item` current and prepares playing it.
    fn prepare(&mut self, item: ItemId) -> Option<PlaybackTask> {
        let locator = self.session.get(item)?.locator.clone();
        self.session.arrive(item);
        self.finished = false;
        self.counted = false;
        Some(
            self.player
                .prepare_track(self.providers.clone(), locator, item),
        )
    }

    fn start(&mut self, task: PlaybackTask, ctx: &Ctx, out: &mut Outbox) {
        self.loaded = false;
        self.set_playing(false, out);
        self.position(TrackTime::default(), true, out);
        if let Some(current) = self.session.current() {
            log::info!("Starting playback: {}", current.locator);
        }
        self.announce(out);
        self.job = Some(ctx.jobs.spawn_stream(PlaybackJob(task)));
        self.update_next();
        self.sync(out);
    }

    fn play(&mut self, item: ItemId, ctx: &Ctx, out: &mut Outbox) {
        if let Some(task) = self.prepare(item) {
            self.start(task, ctx, out);
        }
    }

    /// Plays what comes next; at the end of the context with looping off, stops.
    fn advance(&mut self, manual: bool, ctx: &Ctx, out: &mut Outbox) {
        match self.session.next(&self.loop_status, self.shuffle, manual) {
            Some(item) => self.play(item, ctx, out),
            None => {
                self.stop(out);
                self.finished = true;
            }
        }
    }

    /// Until something plays, the session holds the whole library in the order Play would
    /// use, so the UIs show that order and picking a track in it does not reorder it.
    fn offer_library(&mut self) {
        let library = Query::library();
        let untouched = self
            .session
            .context()
            .is_none_or(|context| *context == library);
        if self.session.current().is_some() || !untouched {
            return;
        }
        let tracks = self.library.read().select(&library);
        self.session
            .replace_context(library, &tracks, None, self.shuffle);
        self.update_next();
    }

    /// Plays from where the session is: the whole library when nothing was chosen yet, the
    /// start of a finished context, or the next item.
    fn resume(&mut self, ctx: &Ctx, out: &mut Outbox) {
        if self.session.context().is_none() {
            self.play_from(&Query::library(), None, ctx, out);
        } else if self.finished {
            if let Some(item) = self.session.restart(self.shuffle) {
                self.play(item, ctx, out);
            }
        } else {
            self.advance(true, ctx, out);
        }
    }

    /// See [`PlayFrom`].
    fn play_from(&mut self, query: &Query, start: Option<&Locator>, ctx: &Ctx, out: &mut Outbox) {
        if self.session.context() == Some(query) {
            if let Some(item) = start.and_then(|start| self.session.find(start)) {
                self.jump(item, 0.0, false, ctx, out);
                return;
            }
        }
        let tracks = self.library.read().select(query);
        let first = self
            .session
            .replace_context(query.clone(), &tracks, start, self.shuffle);
        match first {
            Some(item) => self.play(item, ctx, out),
            None => {
                log::info!("Nothing to play for {:?}", query);
                self.update_next();
                self.sync(out);
            }
        }
    }

    fn current_time(&self) -> TrackTime {
        self.player.time().unwrap_or(self.time)
    }

    fn seek_clamped(&mut self, position: f64, length: f64) {
        let position = if length > 0.0 {
            position.clamp(0.0, length)
        } else {
            position.max(0.0)
        };
        self.player.seek_to(position);
    }

    /// Plays `item` from `position`; just seeks when it is playing already.
    fn jump(&mut self, item: ItemId, position: f64, paused: bool, ctx: &Ctx, out: &mut Outbox) {
        if !position.is_finite() {
            return;
        }
        let current = self.session.current().map(|item| item.id);
        if current == Some(item) && self.player.is_playing() {
            let length = self.time.length;
            self.seek_clamped(position, length);
            return;
        }
        self.session.forget_crossing();
        let Some(task) = self.prepare(item) else {
            return;
        };
        if paused {
            self.player.pause();
        }
        // Queue the seek before starting the worker so it cannot output frames at zero first.
        // The new track's length is unknown yet, so only reject negative positions.
        self.seek_clamped(position, 0.0);
        self.start(task, ctx, out);
    }

    fn seek_to_item(&mut self, item: ItemId, position: f64, ctx: &Ctx, out: &mut Outbox) {
        // Capture the play/pause intent from the existing control before it is replaced; the
        // playback-notification mirror can lag immediate Play/Pause control mutations.
        let paused = !self.player.is_playing() || self.player.is_paused();
        self.jump(item, position, paused, ctx, out);
    }

    fn set_playing(&mut self, playing: bool, out: &mut Outbox) {
        if self.playing != playing {
            self.playing = playing;
            out.emit(PlaybackChanged(playing));
        }
    }

    fn position(&mut self, time: TrackTime, discontinuity: bool, out: &mut Outbox) {
        if let Some(request) = self.pending_seek_request.take() {
            self.seek_request = request;
        }
        self.time = time;
        out.emit(PositionChanged {
            time,
            seek: self.seek_request,
            discontinuity,
        });
    }

    /// Reports the known position as a discontinuity, leaving a pending seek request pending.
    fn report(&self, out: &mut Outbox) {
        out.emit(PositionChanged {
            time: self.time,
            seek: self.seek_request,
            discontinuity: true,
        });
    }

    /// Announces the current item with its track as the library knows it.
    fn announce(&self, out: &mut Outbox) {
        let Some(current) = self.session.current() else {
            return;
        };
        let track = self
            .library
            .read()
            .track(&current.locator)
            .cloned()
            .unwrap_or_else(|| Arc::new(TrackInfo::placeholder(current.locator.clone())));
        out.emit(TrackChanged {
            item: current.id,
            track,
        });
    }

    fn stop(&mut self, out: &mut Outbox) {
        self.player.end_current();
        self.job = None;
        self.loaded = false;
        self.set_playing(false, out);
    }

    /// Counts the current item as played once past its middle.
    fn count_play(&mut self, out: &mut Outbox) {
        if self.counted || self.time.length <= 0.0 || self.time.position < self.time.length / 2.0 {
            return;
        }
        self.counted = true;
        if let Some(current) = self.session.current() {
            out.emit(TrackPlayed {
                locator: current.locator.clone(),
                fingerprint: current.fingerprint,
            });
        }
    }
}

impl Subscriber for QueuePlayer {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<PlayFrom>();
        reg.on::<Enqueue>();
        reg.on::<RemoveQueued>();
        reg.on::<ClearQueued>();
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
        reg.on::<SetShuffle>();
        reg.on::<ToggleShuffle>();
        reg.on::<TracksEnumerated>();
        reg.on::<TrackMetadataLoaded>();
        reg.on::<ScanFinished>();
        reg.on::<SetReplayGain>();
        reg.on::<AppVisibilityChanged>();
        reg.on::<ShutdownRequested>();
        PlaybackJob::subscribe(reg);
    }
}

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

impl Handle<ScanFinished> for QueuePlayer {
    fn handle(&mut self, msg: &ScanFinished, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down || !msg.complete {
            return;
        }
        let library = self.library.clone();
        self.session.reconcile(&library.read());
        self.update_next();
        self.sync(out);
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
                self.failures = 0;
                self.time.length = *length;
                self.report(out);
                self.set_playing(!paused, out);
            }
            PlaybackEvent::Position {
                time,
                revision,
                discontinuity,
            } => {
                if self.player.seek_revision() == *revision {
                    self.position(*time, *discontinuity, out);
                    self.count_play(out);
                }
            }
            PlaybackEvent::Advanced { item, length } => {
                self.session.arrive(*item);
                self.counted = false;
                self.time = TrackTime {
                    position: 0.0,
                    length: *length,
                };
                if let Some(current) = self.session.current() {
                    log::info!("Continuing playback: {}", current.locator);
                }
                self.announce(out);
                self.report(out);
                self.update_next();
                self.sync(out);
            }
            PlaybackEvent::Paused(paused) => {
                self.set_playing(!paused, out);
            }
            PlaybackEvent::Ended => self.advance(false, ctx, out),
            PlaybackEvent::Failed(failure) => {
                log::error!("Playback failed: {failure}");
                self.failures += 1;
                if let Failure::Track(_) = failure {
                    if self.failures < self.session.entries().len() {
                        // Repeating a track that cannot play would only fail again.
                        let manual = self.loop_status == LoopStatus::File;
                        self.advance(manual, ctx, out);
                        return;
                    }
                }
                self.stop(out);
                let time = self.current_time();
                self.position(time, true, out);
            }
        }
    }
}
