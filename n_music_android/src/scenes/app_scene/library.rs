use super::{AppScene, Changes};
use crate::ui::to_track_data;
use n_event_bus::{Ctx, Handle, Outbox, Tagged};
use n_player::jobs::scan::ScanJob;
use n_player::messages::{
    QueueReplaced, ScanFinished, ScanLibrary, SearchChanged, TrackMetadataLoaded, TracksEnumerated,
};

impl Handle<ScanLibrary> for AppScene {
    fn handle(&mut self, msg: &ScanLibrary, ctx: &Ctx, _out: &mut Outbox) {
        self.scan_job = Some(ctx.jobs.spawn_stream(ScanJob {
            settings: msg.settings.clone(),
            internal_dir: msg.internal_dir.clone(),
            check_cache: msg.check_cache,
        }));
        self.loaded = 0;
        self.apply_ui();
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
        self.changes.push(Changes::Tracks(
            enumerated
                .tracks
                .iter()
                .enumerate()
                .map(|(index, track)| to_track_data(track.clone(), index as i32))
                .collect(),
        ));
        out.emit(QueueReplaced {
            path: enumerated.path.clone(),
            names: enumerated.names.clone(),
        });
        self.apply_ui();
    }
}

impl Handle<Tagged<TrackMetadataLoaded>> for AppScene {
    fn handle(&mut self, msg: &Tagged<TrackMetadataLoaded>, _ctx: &Ctx, _out: &mut Outbox) {
        let Some(loaded) = self.scan_job.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        let track = to_track_data(loaded.track.clone(), loaded.index as i32);
        self.changes.push(Changes::Metadata(loaded.index, track));
        self.loaded += 1;
        self.progress_dirty = true;
        self.apply_ui();
    }
}

impl Handle<Tagged<ScanFinished>> for AppScene {
    fn handle(&mut self, msg: &Tagged<ScanFinished>, _ctx: &Ctx, _out: &mut Outbox) {
        let Some(finished) = self.scan_job.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        self.loaded = self.track_count;
        self.progress_dirty = true;
        if let Some(tracks) = finished.tracks.clone() {
            _out.emit(n_player::messages::CacheReady {
                path: finished.path.clone(),
                timestamp: finished.timestamp,
                tracks,
            });
        }
        self.apply_ui();
    }
}

impl Handle<SearchChanged> for AppScene {
    fn handle(&mut self, msg: &SearchChanged, _ctx: &Ctx, _out: &mut Outbox) {
        if self.search.is_empty() && !msg.0.is_empty() {
            self.save_y = true;
        }
        self.search = msg.0.clone();
        self.search_dirty = true;
        self.apply_ui();
    }
}
