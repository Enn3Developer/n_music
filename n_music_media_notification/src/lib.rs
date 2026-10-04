//! OS media integration: MPRIS on Linux, SMTC on Windows, Now Playing on macOS.
//!
//! The crate is a bus subscriber, so hosts only have to register it:
//!
//! ```no_run
//! # use std::sync::Arc;
//! # use n_music_core::library::LibraryPaths;
//! # use n_music_core::source::{LocalProvider, Providers};
//! # fn example(writer: n_event_bus::EventWriter, paths: LibraryPaths) {
//! let providers = Arc::new(Providers::default().with_local(LocalProvider));
//! if let Some(media) =
//!     n_music_media_notification::MediaNotification::new(writer, 1.0, providers, paths)
//! {
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
use n_music_core::library::LibraryPaths;
use n_music_core::messages::{
    LoopStatusChanged, Pause, Play, PlayNext, PlayPrevious, PlaybackChanged, PositionChanged, Seek,
    SetLoopStatus, SetShuffle, SetVolume, ShuffleChanged, TogglePause, TrackChanged, VolumeChanged,
};
use n_music_core::queue::LoopStatus as QueueLoopStatus;
use n_music_core::services::metadata::{MetadataJob, MetadataLoaded, MetadataLoader};
use n_music_core::source::Providers;
use std::any::Any;
use std::sync::{Arc, RwLock};

/// Bridges the event bus to the platform media controls and back.
pub struct MediaNotification {
    state: Arc<RwLock<State>>,
    backend: Arc<dyn Backend>,
    metadata_loader: MetadataLoader,
}

impl MediaNotification {
    /// Attach to the OS media controls. Returns `None` when the platform has no
    /// usable backend (or the backend could not be initialized).
    pub fn new(
        writer: EventWriter,
        volume: f64,
        providers: Arc<Providers>,
        paths: LibraryPaths,
    ) -> Option<Self> {
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
            metadata_loader: MetadataLoader::new(providers, paths),
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
            LoopStatus::Off => QueueLoopStatus::Off,
            LoopStatus::Playlist => QueueLoopStatus::Playlist,
            LoopStatus::File => QueueLoopStatus::File,
        })),
        MediaEvent::SetShuffle(shuffle) => writer.emit(SetShuffle(shuffle)),
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
        reg.on::<ShuffleChanged>();
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
            QueueLoopStatus::Off => LoopStatus::Off,
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
            self.metadata_loader.load(msg.locator.clone(), ctx);
        }
    }
}

impl Handle<Tagged<MetadataLoaded>> for MediaNotification {
    fn handle(&mut self, msg: &Tagged<MetadataLoaded>, ctx: &Ctx, _out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let Some(track) = self.metadata_loader.take(msg) else {
            return;
        };
        {
            let mut state = self.state.write().unwrap();
            state.title = track.title.clone();
            state.artist = track.artist();
            state.length = track.length;
            state.cover = track.cover.clone();
        }
        self.update(Change::Metadata);
    }
}

impl Handle<ShuffleChanged> for MediaNotification {
    fn handle(&mut self, msg: &ShuffleChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.state.write().unwrap().shuffle = msg.0;
        self.update(Change::Shuffle);
    }
}

impl Handle<ShutdownRequested> for MediaNotification {
    fn handle(&mut self, _: &ShutdownRequested, _: &Ctx, out: &mut Outbox) {
        self.metadata_loader.cancel();
        self.state.write().unwrap().playing = false;
        self.update(Change::Playback);
        out.shutdown_ready();
    }
}
