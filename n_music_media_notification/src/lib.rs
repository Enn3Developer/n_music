//! OS media integration: MPRIS on Linux, SMTC on Windows, Now Playing on macOS.
//!
//! The crate is a bus subscriber, so hosts only have to register it:
//!
//! ```no_run
//! # fn example(writer: n_event_bus::EventWriter) {
//! if let Some(media) = n_music_media_notification::MediaNotification::new(writer, 1.0) {
//!     // app.register_subscriber(media);
//! }
//! # }
//! ```

mod platform;
mod state;

use crate::state::{Backend, Change, Emit, LoopStatus, MediaEvent, State};
use n_event_bus::{
    Ctx, EventWriter, Handle, Outbox, Registrar, ShutdownRequested, Subscriber, Tagged,
};
use n_music_core::messages::{
    LoopStatusChanged, Pause, Play, PlayNext, PlayPrevious, PlaybackChanged, PositionChanged, Seek,
    SetLoopStatus, SetVolume, TogglePause, TrackChanged, VolumeChanged,
};
use n_music_core::queue::LoopStatus as QueueLoopStatus;
use n_music_core::services::metadata::{MetadataJob, MetadataLoaded, MetadataLoader};
use std::any::Any;
use std::sync::{Arc, RwLock};
use tempfile::NamedTempFile;

/// Bridges the event bus to the platform media controls and back.
pub struct MediaNotification {
    state: Arc<RwLock<State>>,
    backend: Arc<dyn Backend>,
    metadata_loader: MetadataLoader,
    covers: Vec<NamedTempFile>,
}

impl MediaNotification {
    /// Attach to the OS media controls. Returns `None` when the platform has no
    /// usable backend (or the backend could not be initialized).
    pub fn new(writer: EventWriter, volume: f64) -> Option<Self> {
        let state = Arc::new(RwLock::new(State {
            volume,
            ..State::default()
        }));
        let emit_writer = writer.clone();
        let emit: Emit = Arc::new(move |event| dispatch(&emit_writer, event));
        let backend = platform::new_controls(emit, state.clone())?;

        Some(Self {
            state,
            backend: Arc::from(backend),
            metadata_loader: MetadataLoader::default(),
            covers: Vec::new(),
        })
    }

    fn update(&self, change: Change) {
        let state = self.state.read().unwrap();
        self.backend.update(&state, change);
    }
}

fn dispatch(writer: &EventWriter, event: MediaEvent) {
    match event {
        MediaEvent::Play => writer.emit(Play),
        MediaEvent::Pause => writer.emit(Pause),
        MediaEvent::TogglePause => writer.emit(TogglePause),
        MediaEvent::Next => writer.emit(PlayNext),
        MediaEvent::Previous => writer.emit(PlayPrevious),
        MediaEvent::SeekRelative(seconds) => writer.emit(Seek::Relative(seconds)),
        MediaEvent::SeekAbsolute(seconds) => writer.emit(Seek::Absolute(seconds)),
        MediaEvent::SetVolume(volume) => writer.emit(SetVolume(volume)),
        MediaEvent::SetLoopStatus(loop_status) => writer.emit(SetLoopStatus(match loop_status {
            LoopStatus::Playlist => QueueLoopStatus::Playlist,
            LoopStatus::File => QueueLoopStatus::File,
        })),
    }
}

impl Subscriber for MediaNotification {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        MetadataJob::subscribe(reg);
        reg.on::<PlaybackChanged>();
        reg.on::<TrackChanged>();
        reg.on::<VolumeChanged>();
        reg.on::<PositionChanged>();
        reg.on::<LoopStatusChanged>();
        reg.on::<ShutdownRequested>();
    }
}

impl Handle<PlaybackChanged> for MediaNotification {
    fn handle(&mut self, msg: &PlaybackChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.state.write().unwrap().playing = msg.0;
        self.update(Change::Playback);
    }
}

impl Handle<VolumeChanged> for MediaNotification {
    fn handle(&mut self, msg: &VolumeChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.state.write().unwrap().volume = msg.0;
        self.update(Change::Volume);
    }
}

impl Handle<LoopStatusChanged> for MediaNotification {
    fn handle(&mut self, msg: &LoopStatusChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.state.write().unwrap().loop_status = match msg.0 {
            QueueLoopStatus::Playlist => LoopStatus::Playlist,
            QueueLoopStatus::File => LoopStatus::File,
        };
        self.update(Change::Loop);
    }
}

impl Handle<PositionChanged> for MediaNotification {
    fn handle(&mut self, msg: &PositionChanged, _ctx: &Ctx, _out: &mut Outbox) {
        {
            let mut state = self.state.write().unwrap();
            state.position = msg.0.position;
            state.length = msg.0.length;
        }
        self.update(Change::Position {
            discontinuity: msg.2,
        });
    }
}

impl Handle<TrackChanged> for MediaNotification {
    fn handle(&mut self, msg: &TrackChanged, ctx: &Ctx, _out: &mut Outbox) {
        if !ctx.shutting_down {
            self.metadata_loader.load(msg.path.clone(), ctx);
        }
    }
}

impl Handle<Tagged<MetadataLoaded>> for MediaNotification {
    fn handle(&mut self, msg: &Tagged<MetadataLoaded>, ctx: &Ctx, _out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let Some(loaded) = self.metadata_loader.take(msg) else {
            return;
        };
        let cover = loaded.cover;
        {
            let mut state = self.state.write().unwrap();
            state.title = loaded.metadata.title;
            state.artist = loaded.metadata.artist;
            state.length = loaded.metadata.time.length;
            state.cover = cover.as_ref().map(|file| file.path().to_path_buf());
        }
        if let Some(cover) = cover {
            self.covers.push(cover);
            // Keep one generation of covers alive: an already queued backend
            // update may still need to read the previous file.
            if self.covers.len() > 2 {
                self.covers.remove(0);
            }
        }
        self.update(Change::Metadata);
    }
}

impl Handle<ShutdownRequested> for MediaNotification {
    fn handle(&mut self, _: &ShutdownRequested, _: &Ctx, out: &mut Outbox) {
        self.metadata_loader = MetadataLoader::default();
        self.state.write().unwrap().playing = false;
        self.update(Change::Playback);
        out.shutdown_ready();
    }
}
