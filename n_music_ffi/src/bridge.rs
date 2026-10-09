//! What the core announces, copied for Kotlin. [`KotlinBridge`] listens on the bus like any
//! other subscriber and queues a [`CoreEvent`] for each event; Kotlin takes them with
//! `Core::next_event`, so the bus thread never waits on Kotlin.

use crate::rows::{QueueRow, TrackRow};
use n_event_bus::{Ctx, Handle, Outbox, Registrar, Subscriber};
use n_music_core::library::catalog::Library;
use n_music_core::messages::{
    LibraryRenamed, LibraryRootsChanged, LoopStatusChanged, OutputDevices, PlaybackChanged,
    PlaylistRejected, PlaylistSummary, PlaylistsChanged, PositionChanged, QueueChanged,
    ScanFinished, ScanProgress, ShuffleChanged, StreamCacheChanged, TrackChanged,
    TrackMetadataLoaded, TrackPlayed, TracksEnumerated, VolumeChanged,
};
use n_music_core::queue::{ItemId, LoopStatus};
use n_music_core::settings::OutputDevice;
use n_music_core::source::Locator;
use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Something the core announced. Each variant follows the event message of the same name in
/// `n_music_core::messages`, except [`CoreEvent::LibraryChanged`].
#[derive(uniffi::Enum)]
pub enum CoreEvent {
    PlaybackChanged {
        playing: bool,
    },
    /// The current item; sent again when its track's metadata loads.
    TrackChanged {
        item: ItemId,
        track: TrackRow,
    },
    PositionChanged {
        position: f64,
        length: f64,
        /// The last `Seek.Tracked` request that applied, 0 before any.
        seek: u64,
        /// The position jumped (seek, new track, pause) rather than advanced with playback.
        discontinuity: bool,
    },
    VolumeChanged {
        volume: f64,
    },
    LoopStatusChanged {
        status: LoopStatus,
    },
    ShuffleChanged {
        enabled: bool,
    },
    QueueChanged {
        entries: Vec<QueueRow>,
    },
    /// A track was listened to past its middle.
    TrackPlayed {
        locator: Locator,
    },
    PlaylistsChanged {
        playlists: Vec<PlaylistSummary>,
    },
    PlaylistRejected {
        reason: String,
    },
    /// The library's tracks or names changed: query again. Bursts while a scan reads tracks
    /// come as one until Kotlin takes it.
    LibraryChanged {
        /// The tracks scans read since launch, as of when Kotlin took this. Less the `read` of
        /// the last `ScanProgress`, what the running scan read since that report.
        read: u64,
    },
    ScanProgress {
        libraries: Vec<Locator>,
        found: u64,
        pending: u64,
        /// The tracks scans read since launch, as of this report.
        read: u64,
    },
    ScanFinished {
        libraries: Vec<Locator>,
        complete: bool,
    },
    LibraryRootsChanged {
        roots: Vec<Locator>,
    },
    StreamCacheChanged {
        enabled: bool,
        limit: u64,
        used: u64,
    },
    OutputDevices {
        devices: Vec<OutputDevice>,
    },
}

pub struct KotlinBridge {
    events: flume::Sender<CoreEvent>,
    library: Library,
    /// A [`CoreEvent::LibraryChanged`] is queued and Kotlin has not taken it yet.
    library_changed: Arc<AtomicBool>,
    /// See [`CoreEvent::LibraryChanged`].
    scan_read: Arc<AtomicU64>,
}

impl KotlinBridge {
    pub fn new(
        events: flume::Sender<CoreEvent>,
        library: Library,
        library_changed: Arc<AtomicBool>,
        scan_read: Arc<AtomicU64>,
    ) -> Self {
        Self {
            events,
            library,
            library_changed,
            scan_read,
        }
    }

    fn send(&self, event: CoreEvent) {
        // Fails only once the core is gone, when nobody listens anymore.
        let _ = self.events.send(event);
    }

    pub(crate) fn library_changed(&self) {
        if !self.library_changed.swap(true, Ordering::AcqRel) {
            // `read` is filled in when Kotlin takes it.
            self.send(CoreEvent::LibraryChanged { read: 0 });
        }
    }
}

impl Subscriber for KotlinBridge {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<PlaybackChanged>();
        reg.on::<TrackChanged>();
        reg.on::<PositionChanged>();
        reg.on::<VolumeChanged>();
        reg.on::<LoopStatusChanged>();
        reg.on::<ShuffleChanged>();
        reg.on::<QueueChanged>();
        reg.on::<TrackPlayed>();
        reg.on::<PlaylistsChanged>();
        reg.on::<PlaylistRejected>();
        reg.on::<TracksEnumerated>();
        reg.on::<TrackMetadataLoaded>();
        reg.on::<LibraryRenamed>();
        reg.on::<ScanProgress>();
        reg.on::<ScanFinished>();
        reg.on::<LibraryRootsChanged>();
        reg.on::<StreamCacheChanged>();
        reg.on::<OutputDevices>();
    }
}

impl Handle<PlaybackChanged> for KotlinBridge {
    fn handle(&mut self, msg: &PlaybackChanged, _: &Ctx, _: &mut Outbox) {
        self.send(CoreEvent::PlaybackChanged { playing: msg.0 });
    }
}

impl Handle<TrackChanged> for KotlinBridge {
    fn handle(&mut self, msg: &TrackChanged, _: &Ctx, _: &mut Outbox) {
        let track = TrackRow::new(&msg.track, &self.library.read());
        self.send(CoreEvent::TrackChanged {
            item: msg.item,
            track,
        });
    }
}

impl Handle<PositionChanged> for KotlinBridge {
    fn handle(&mut self, msg: &PositionChanged, _: &Ctx, _: &mut Outbox) {
        self.send(CoreEvent::PositionChanged {
            position: msg.time.position,
            length: msg.time.length,
            seek: msg.seek,
            discontinuity: msg.discontinuity,
        });
    }
}

impl Handle<VolumeChanged> for KotlinBridge {
    fn handle(&mut self, msg: &VolumeChanged, _: &Ctx, _: &mut Outbox) {
        self.send(CoreEvent::VolumeChanged { volume: msg.0 });
    }
}

impl Handle<LoopStatusChanged> for KotlinBridge {
    fn handle(&mut self, msg: &LoopStatusChanged, _: &Ctx, _: &mut Outbox) {
        self.send(CoreEvent::LoopStatusChanged {
            status: msg.0.clone(),
        });
    }
}

impl Handle<ShuffleChanged> for KotlinBridge {
    fn handle(&mut self, msg: &ShuffleChanged, _: &Ctx, _: &mut Outbox) {
        self.send(CoreEvent::ShuffleChanged { enabled: msg.0 });
    }
}

impl Handle<QueueChanged> for KotlinBridge {
    fn handle(&mut self, msg: &QueueChanged, _: &Ctx, _: &mut Outbox) {
        let entries = {
            let catalog = self.library.read();
            msg.entries
                .iter()
                .map(|entry| QueueRow::new(entry, &catalog))
                .collect()
        };
        self.send(CoreEvent::QueueChanged { entries });
    }
}

impl Handle<TrackPlayed> for KotlinBridge {
    fn handle(&mut self, msg: &TrackPlayed, _: &Ctx, _: &mut Outbox) {
        self.send(CoreEvent::TrackPlayed {
            locator: msg.locator.clone(),
        });
    }
}

impl Handle<PlaylistsChanged> for KotlinBridge {
    fn handle(&mut self, msg: &PlaylistsChanged, _: &Ctx, _: &mut Outbox) {
        let playlists = msg
            .0
            .iter()
            .map(|playlist| PlaylistSummary {
                id: playlist.id,
                name: playlist.name.clone(),
                smart: playlist.smart,
            })
            .collect();
        self.send(CoreEvent::PlaylistsChanged { playlists });
    }
}

impl Handle<PlaylistRejected> for KotlinBridge {
    fn handle(&mut self, msg: &PlaylistRejected, _: &Ctx, _: &mut Outbox) {
        self.send(CoreEvent::PlaylistRejected {
            reason: msg.0.clone(),
        });
    }
}

impl Handle<TracksEnumerated> for KotlinBridge {
    fn handle(&mut self, _: &TracksEnumerated, _: &Ctx, _: &mut Outbox) {
        self.library_changed();
    }
}

impl Handle<TrackMetadataLoaded> for KotlinBridge {
    fn handle(&mut self, _: &TrackMetadataLoaded, _: &Ctx, _: &mut Outbox) {
        self.scan_read.fetch_add(1, Ordering::AcqRel);
        self.library_changed();
    }
}

impl Handle<LibraryRenamed> for KotlinBridge {
    fn handle(&mut self, _: &LibraryRenamed, _: &Ctx, _: &mut Outbox) {
        self.library_changed();
    }
}

impl Handle<ScanProgress> for KotlinBridge {
    fn handle(&mut self, msg: &ScanProgress, _: &Ctx, _: &mut Outbox) {
        // `pending` counts what is left to read from here. The count goes on rather than
        // starting over, so a `LibraryChanged` Kotlin takes after this but queued before it
        // cannot count against the new report.
        self.send(CoreEvent::ScanProgress {
            libraries: msg.libraries.clone(),
            found: msg.found as u64,
            pending: msg.pending as u64,
            read: self.scan_read.load(Ordering::Acquire),
        });
    }
}

impl Handle<ScanFinished> for KotlinBridge {
    fn handle(&mut self, msg: &ScanFinished, _: &Ctx, _: &mut Outbox) {
        self.send(CoreEvent::ScanFinished {
            libraries: msg.libraries.clone(),
            complete: msg.complete,
        });
    }
}

impl Handle<LibraryRootsChanged> for KotlinBridge {
    fn handle(&mut self, msg: &LibraryRootsChanged, _: &Ctx, _: &mut Outbox) {
        self.send(CoreEvent::LibraryRootsChanged {
            roots: msg.0.clone(),
        });
    }
}

impl Handle<StreamCacheChanged> for KotlinBridge {
    fn handle(&mut self, msg: &StreamCacheChanged, _: &Ctx, _: &mut Outbox) {
        self.send(CoreEvent::StreamCacheChanged {
            enabled: msg.enabled,
            limit: msg.limit,
            used: msg.used,
        });
    }
}

impl Handle<OutputDevices> for KotlinBridge {
    fn handle(&mut self, msg: &OutputDevices, _: &Ctx, _: &mut Outbox) {
        self.send(CoreEvent::OutputDevices {
            devices: msg.0.clone(),
        });
    }
}
