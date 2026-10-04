use super::covers::CoverJob;
use super::{AppScene, Changes};
use crate::ui::to_track_data;
use n_event_bus::{Ctx, Handle, Outbox, Tagged};
use n_music_core::jobs::scan::ScanJob;
use n_music_core::messages::{
    QueueReplaced, ScanFinished, ScanRequested, SearchChanged, TrackMetadataLoaded,
    TracksEnumerated,
};

impl Handle<ScanRequested> for AppScene {
    fn handle(&mut self, msg: &ScanRequested, ctx: &Ctx, _out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        self.scan_job = Some(ctx.jobs.spawn_stream(ScanJob {
            roots: self.library.get().libraries.clone(),
            paths: self.paths.clone(),
            check_cache: msg.check_cache,
            providers: self.providers.clone(),
        }));
        self.loaded = 0;
        self.apply_ui();
    }
}

impl Handle<Tagged<TracksEnumerated>> for AppScene {
    fn handle(&mut self, msg: &Tagged<TracksEnumerated>, ctx: &Ctx, out: &mut Outbox) {
        let Some(enumerated) = self.scan_job.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        self.track_count = enumerated.tracks.len();
        self.covers.retain(&enumerated.tracks);
        let mut pending = vec![];
        self.loaded = 0;
        self.progress_dirty = true;
        self.changes.push(Changes::Tracks(
            enumerated
                .tracks
                .iter()
                .enumerate()
                .map(|(index, track)| {
                    let cover = track.cover.as_ref().and_then(|path| {
                        let cover = self.covers.get(path);
                        if cover.is_none() {
                            pending.push((index, path.clone()));
                        }
                        cover
                    });
                    to_track_data(track, index as i32, cover)
                })
                .collect(),
        ));
        self.cover_job = (!pending.is_empty()).then(|| ctx.jobs.spawn_stream(CoverJob(pending)));
        out.emit(QueueReplaced {
            tracks: enumerated
                .tracks
                .iter()
                .map(|track| track.locator.clone())
                .collect(),
        });
        self.apply_ui();
    }
}

impl Handle<Tagged<TrackMetadataLoaded>> for AppScene {
    fn handle(&mut self, msg: &Tagged<TrackMetadataLoaded>, _ctx: &Ctx, _out: &mut Outbox) {
        let Some(loaded) = self.scan_job.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        // Freshly scanned tracks arrive at scan speed, so decoding one cover here is cheap.
        let cover = loaded
            .track
            .cover
            .clone()
            .and_then(|path| self.covers.load(path));
        let track = to_track_data(&loaded.track, loaded.index as i32, cover);
        self.changes.push(Changes::Metadata(loaded.index, track));
        self.loaded += 1;
        self.progress_dirty = true;
        self.apply_ui();
    }
}

impl Handle<Tagged<ScanFinished>> for AppScene {
    fn handle(&mut self, msg: &Tagged<ScanFinished>, _ctx: &Ctx, _out: &mut Outbox) {
        if self
            .scan_job
            .as_ref()
            .and_then(|job| job.open(msg))
            .is_none()
        {
            return;
        }
        self.loaded = self.track_count;
        self.progress_dirty = true;
        self.apply_ui();
    }
}

impl Handle<SearchChanged> for AppScene {
    fn handle(&mut self, msg: &SearchChanged, ctx: &Ctx, _out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        if self.search.is_empty() && !msg.0.is_empty() {
            self.save_y = true;
        }
        self.search = msg.0.clone();
        self.search_dirty = true;
        self.apply_ui();
    }
}
