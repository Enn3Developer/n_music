use crate::messages::{
    AppVisibilityChanged, LoopStatusChanged, Pause, Play, PlayNext, PlayPrevious, PlayTrack,
    PlaybackChanged, PlaybackFailed, PositionChanged, QueueReplaced, Seek, SetLoopStatus,
    SetVolume, Shutdown, TrackChanged, TrackEnded, TogglePause, VolumeChanged,
};
use crate::player::{PlaybackTask, Player};
use crate::TrackTime;
use crate::{remove_ext, strip_absolute_path};
use n_event_bus::{Ctx, Handle, Outbox, Registrar, Subscriber};
use rand::prelude::SliceRandom;
use rand::rng;
use std::any::Any;
use std::cmp::PartialEq;
use std::io;
use std::io::ErrorKind;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub enum LoopStatus {
    #[default]
    Playlist,
    File,
}

pub struct QueuePlayer {
    queue: Vec<Arc<str>>,
    path: String,
    player: Player,
    index: usize,
    loop_status: LoopStatus,
    seek_revision: i32,
    pending_seek_revision: Arc<Mutex<Option<i32>>>,
}

impl Default for QueuePlayer {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl QueuePlayer {
    pub fn new(path: String) -> Self {
        let mut player = Player::new(1.0, 1.0);
        player.set_progress_interval(Some(Duration::from_millis(250)));

        QueuePlayer {
            queue: vec![],
            player,
            index: usize::MAX - 1,
            path,
            loop_status: LoopStatus::Playlist,
            seek_revision: 0,
            pending_seek_revision: Arc::new(Mutex::new(None)),
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

    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn set_path(&mut self, path: String) {
        self.path = path;
    }

    pub fn set_loop_status(&mut self, loop_status: LoopStatus) {
        self.loop_status = loop_status;
    }

    pub fn loop_status(&self) -> LoopStatus {
        self.loop_status.clone()
    }

    pub fn get_path_for_file(&self, i: usize) -> Option<PathBuf> {
        Some(PathBuf::from(&self.path).join(self.queue.get(i)?.as_ref()))
    }

    pub fn queue(&self) -> &[Arc<str>] {
        &self.queue
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.path.shrink_to_fit();
        self.queue.shrink_to_fit();
    }

    #[inline]
    pub fn add<P: Into<Arc<str>>>(&mut self, path: P) {
        self.queue.push(path.into());
    }

    pub fn add_all<P: Into<String>>(&mut self, paths: impl IntoIterator<Item = P>) {
        self.queue.append(
            &mut paths
                .into_iter()
                .map(|p| strip_absolute_path(p.into()).into())
                .collect::<Vec<Arc<str>>>(),
        );
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

    pub fn current_track_name(&self) -> Option<Arc<str>> {
        self.queue.get(self.index).map(|t| t.clone())
    }

    pub fn prepare_index(&mut self, index: usize) -> io::Result<PlaybackTask> {
        if self.queue.is_empty() {
            return Err(ErrorKind::NotFound.into());
        }
        self.index = index % self.len();
        let path = self
            .get_path_for_file(self.index)
            .ok_or(ErrorKind::NotFound)?;
        Ok(self.player.prepare_path(path))
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

    pub fn get_index_from_track_name(&self, name: &str) -> Option<usize> {
        self.queue
            .iter()
            .map(|t| remove_ext(t.as_ref()))
            .enumerate()
            .find(|(_i, t)| t == name)
            .map(|(i, _t)| i)
    }
}

impl QueuePlayer {
    fn position(&mut self, time: TrackTime, discontinuity: bool, out: &mut Outbox) {
        out.emit(PositionChanged(time, self.seek_revision, discontinuity));
    }

    fn stop(&mut self, out: &mut Outbox) {
        self.end_current();
        out.emit(PlaybackChanged(false));
    }

    fn start(&mut self, task: io::Result<PlaybackTask>, ctx: &Ctx, out: &mut Outbox) {
        let Ok(task) = task else {
            return;
        };
        out.emit(PlaybackChanged(false));
        out.emit(PositionChanged(
            TrackTime::default(),
            self.seek_revision,
            true,
        ));
        let index = self.index();
        if let (Some(path), Some(name)) = (self.get_path_for_file(index), self.current_track_name())
        {
            out.emit(TrackChanged { index, path, name });
        }
        task.spawn(ctx.jobs.writer().clone(), self.pending_seek_revision.clone());
    }

    fn advance(&mut self, force: bool, ctx: &Ctx, out: &mut Outbox) {
        let task = self.prepare_next(force);
        self.start(task, ctx, out);
    }

    fn current_time(&self) -> TrackTime {
        self.get_time().unwrap_or_default()
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
        reg.on::<Seek>();
        reg.on::<SetVolume>();
        reg.on::<SetLoopStatus>();
        reg.on::<QueueReplaced>();
        reg.on::<AppVisibilityChanged>();
        reg.on::<Shutdown>();
        reg.on::<TrackEnded>();
        reg.on::<PlaybackFailed>();
    }
}

impl Handle<PlayTrack> for QueuePlayer {
    fn handle(&mut self, msg: &PlayTrack, ctx: &Ctx, out: &mut Outbox) {
        let task = self.prepare_index(msg.0);
        self.start(task, ctx, out);
    }
}

impl Handle<PlayNext> for QueuePlayer {
    fn handle(&mut self, _msg: &PlayNext, ctx: &Ctx, out: &mut Outbox) {
        self.advance(true, ctx, out);
    }
}

impl Handle<PlayPrevious> for QueuePlayer {
    fn handle(&mut self, _msg: &PlayPrevious, ctx: &Ctx, out: &mut Outbox) {
        if self.get_time().is_some_and(|time| time.position > 3.0) {
            self.seek_to(0, 0.0);
        } else {
            let task = self.prepare_previous();
            self.start(task, ctx, out);
        }
    }
}

impl Handle<TogglePause> for QueuePlayer {
    fn handle(&mut self, _msg: &TogglePause, ctx: &Ctx, out: &mut Outbox) {
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
        if self.is_playing() {
            self.unpause();
        } else {
            self.advance(true, ctx, out);
        }
    }
}

impl Handle<Seek> for QueuePlayer {
    fn handle(&mut self, msg: &Seek, _ctx: &Ctx, out: &mut Outbox) {
        let position = match msg {
            Seek::FromUi { position, revision } => {
                *self.pending_seek_revision.lock().unwrap() = Some(*revision);
                self.seek_revision = *revision;
                *position
            }
            Seek::Absolute(position) => *position,
            Seek::Relative(offset) => self.current_time().position + offset,
        };
        if self.is_playing() && position.is_finite() {
            let length = self.get_time().map_or(0.0, |time| time.length);
            let position = if length > 0.0 {
                position.clamp(0.0, length)
            } else {
                position.max(0.0)
            };
            self.seek_to(position.trunc() as u64, position.fract());
            return;
        }
        let time = self.current_time();
        self.position(time, true, out);
    }
}

impl Handle<SetVolume> for QueuePlayer {
    fn handle(&mut self, msg: &SetVolume, _ctx: &Ctx, out: &mut Outbox) {
        if !msg.0.is_finite() {
            return;
        }
        let volume = msg.0.clamp(0.0, 1.0);
        if self.get_volume() == volume as f32 {
            return;
        }
        self.set_volume(volume as f32);
        out.emit(VolumeChanged(volume));
    }
}

impl Handle<SetLoopStatus> for QueuePlayer {
    fn handle(&mut self, msg: &SetLoopStatus, _ctx: &Ctx, out: &mut Outbox) {
        if self.loop_status() != msg.0 {
            self.set_loop_status(msg.0.clone());
            out.emit(LoopStatusChanged(self.loop_status()));
        }
    }
}

impl Handle<QueueReplaced> for QueuePlayer {
    fn handle(&mut self, msg: &QueueReplaced, _ctx: &Ctx, out: &mut Outbox) {
        self.stop(out);
        self.clear();
        self.set_path(msg.path.clone());
        self.add_all(msg.names.clone());
        self.position(TrackTime::default(), true, out);
    }
}

impl Handle<AppVisibilityChanged> for QueuePlayer {
    fn handle(&mut self, msg: &AppVisibilityChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.set_progress_interval(msg.0.then_some(Duration::from_millis(250)));
    }
}

impl Handle<Shutdown> for QueuePlayer {
    fn handle(&mut self, _msg: &Shutdown, _ctx: &Ctx, out: &mut Outbox) {
        self.stop(out);
    }
}

impl Handle<TrackEnded> for QueuePlayer {
    fn handle(&mut self, _msg: &TrackEnded, ctx: &Ctx, out: &mut Outbox) {
        self.advance(false, ctx, out);
    }
}

impl Handle<PlaybackFailed> for QueuePlayer {
    fn handle(&mut self, msg: &PlaybackFailed, _ctx: &Ctx, out: &mut Outbox) {
        eprintln!("error playing track: {}", msg.0);
        self.stop(out);
        let time = self.current_time();
        self.position(time, true, out);
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
