use crate::library::covers::CoverStore;
use crate::library::LibraryPaths;
use crate::music_track::MusicTrack;
use crate::source::{Locator, Providers};
use crate::{Track, TrackInfo};
use n_event_bus::{job_emits, Ctx, EventWriter, Job, JobToken, RunningJob, Tagged};
use std::sync::Arc;

/// Looks up a track's metadata: from the library database when it was scanned, otherwise by
/// reading the file (its cover then goes to the cover store too).
pub struct MetadataJob {
    pub providers: Arc<Providers>,
    pub paths: LibraryPaths,
    pub locator: Locator,
}

pub struct MetadataLoaded(pub Option<Track>);

job_emits!(MetadataJob => Tagged<MetadataLoaded>);

impl Job for MetadataJob {
    fn run(self, tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        let info = self.stored().or_else(|| self.read());
        writer.emit_tagged(tag, MetadataLoaded(info.map(Arc::new)));
    }
}

impl MetadataJob {
    fn stored(&self) -> Option<TrackInfo> {
        let db = self
            .paths
            .open_db()
            .inspect_err(|error| log::warn!("Could not open the library database: {error}"))
            .ok()?;
        db.track(&self.locator)
            .inspect_err(|error| log::warn!("Could not look up {}: {error}", self.locator))
            .ok()
            .flatten()
            .filter(|info| info.cover.as_ref().is_none_or(|cover| cover.is_file()))
    }

    fn read(&self) -> Option<TrackInfo> {
        let locator = &self.locator;
        let (mut info, cover) = MusicTrack::new(self.providers.as_ref(), locator)
            .read_info()
            .inspect_err(|error| log::warn!("Could not read metadata for {locator}: {error}"))
            .ok()?;
        info.cover =
            cover.and_then(|data| CoverStore::open(&self.paths.covers).store(&data, locator));
        Some(info)
    }
}

pub struct MetadataLoader {
    providers: Arc<Providers>,
    paths: LibraryPaths,
    job: Option<RunningJob>,
}

impl MetadataLoader {
    pub fn new(providers: Arc<Providers>, paths: LibraryPaths) -> Self {
        Self {
            providers,
            paths,
            job: None,
        }
    }

    pub fn load(&mut self, locator: Locator, ctx: &Ctx) {
        self.job = Some(ctx.jobs.spawn_oneshot(MetadataJob {
            providers: self.providers.clone(),
            paths: self.paths.clone(),
            locator,
        }));
    }

    /// Drops the pending load, if any; its result will be ignored
    pub fn cancel(&mut self) {
        self.job = None;
    }

    pub fn take(&mut self, message: &Tagged<MetadataLoaded>) -> Option<Track> {
        let loaded = self.job.as_ref()?.open(message)?;
        let track = loaded.0.clone();
        self.job = None;
        track
    }
}
