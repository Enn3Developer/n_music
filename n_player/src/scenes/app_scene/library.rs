use super::{AppScene, Changes};
use crate::jobs::scan::ScanJob;
use crate::messages::{
    QueueReplaced, ScanFinished, ScanRequested, SearchChanged, TrackMetadataLoaded,
    TracksEnumerated,
};
use crate::TrackData;
use n_event_bus::{Ctx, Handle, Outbox, Tagged};

impl Handle<ScanRequested> for AppScene {
    fn handle(&mut self, msg: &ScanRequested, ctx: &Ctx, _out: &mut Outbox) {
        self.scan_job = Some(ctx.jobs.spawn_stream(ScanJob {
            settings: self.settings.clone(),
            platform: self.platform.clone(),
            check_cache: msg.check_cache,
        }));
        self.loaded = 0;
    }
}

impl Handle<Tagged<TracksEnumerated>> for AppScene {
    fn handle(&mut self, msg: &Tagged<TracksEnumerated>, _ctx: &Ctx, out: &mut Outbox) {
        let Some(enumerated) = self.scan_job.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        self.track_count = enumerated.names.len();
        self.loaded = 0;
        self.progress_dirty = true;
        self.changes
            .push(Changes::Tracks(enumerated.tracks.clone()));
        out.emit(QueueReplaced {
            path: enumerated.path.clone(),
            names: enumerated.names.clone(),
        });
    }
}

impl Handle<Tagged<TrackMetadataLoaded>> for AppScene {
    fn handle(&mut self, msg: &Tagged<TrackMetadataLoaded>, _ctx: &Ctx, _out: &mut Outbox) {
        let Some(loaded) = self.scan_job.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        let mut track: TrackData = loaded.track.clone().into();
        track.index = loaded.index as i32;
        self.changes.push(Changes::Metadata(loaded.index, track));
        self.loaded += 1;
        self.progress_dirty = true;
    }
}

impl Handle<Tagged<ScanFinished>> for AppScene {
    fn handle(&mut self, msg: &Tagged<ScanFinished>, _ctx: &Ctx, _out: &mut Outbox) {
        let Some(finished) = self.scan_job.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        self.progress_dirty = true;
        if let Some(tracks) = finished.tracks.clone() {
            let settings = self.settings.clone();
            let platform = self.platform.clone();
            tokio::spawn(async move {
                let internal_dir = platform.internal_dir().await;
                let mut settings = settings.write().await;
                settings.add_tracks(internal_dir.clone(), tracks).await;
                settings.save_timestamp().await;
                settings.save(internal_dir).await;
            });
        }
    }
}

impl Handle<SearchChanged> for AppScene {
    fn handle(&mut self, msg: &SearchChanged, _ctx: &Ctx, _out: &mut Outbox) {
        if self.search.is_empty() && !msg.0.is_empty() {
            self.save_y = true;
        }
        self.search = msg.0.clone();
        self.search_dirty = true;
    }
}
