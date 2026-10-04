use super::covers::CoverJob;
use super::{AppScene, Changes, TrackClicked};
use crate::messages::SearchChanged;
use crate::ui::to_track_data;
use n_event_bus::{Ctx, Handle, Outbox};
use n_music_core::library::query::Query;
use n_music_core::messages::{
    PlayFrom, ScanFinished, ScanRequested, TrackMetadataLoaded, TracksEnumerated,
};

impl Handle<ScanRequested> for AppScene {
    fn handle(&mut self, _msg: &ScanRequested, ctx: &Ctx, _out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        self.loaded = 0;
        self.apply_ui();
    }
}

impl Handle<TrackClicked> for AppScene {
    fn handle(&mut self, msg: &TrackClicked, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        // The interim UI plays the whole library, as it always did.
        if let Some(locator) = self.locators.get(msg.0) {
            out.emit(PlayFrom {
                query: Query::library(),
                start: Some(locator.clone()),
            });
        }
    }
}

impl Handle<TracksEnumerated> for AppScene {
    fn handle(&mut self, enumerated: &TracksEnumerated, ctx: &Ctx, _out: &mut Outbox) {
        self.track_count = enumerated.tracks.len();
        self.locators = enumerated
            .tracks
            .iter()
            .map(|track| track.locator.clone())
            .collect();
        self.index_of = self
            .locators
            .iter()
            .enumerate()
            .map(|(index, locator)| (locator.clone(), index))
            .collect();
        // Rows start in library order; the queue sends the play order next.
        self.row_of = (0..self.track_count).collect();
        self.playing_index = self.playing_row();
        self.dirty = true;
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
        self.apply_ui();
    }
}

impl Handle<TrackMetadataLoaded> for AppScene {
    fn handle(&mut self, loaded: &TrackMetadataLoaded, _ctx: &Ctx, _out: &mut Outbox) {
        let Some(&index) = self.index_of.get(&loaded.track.locator) else {
            return;
        };
        // Freshly scanned tracks arrive at scan speed, so decoding one cover here is cheap.
        let cover = loaded
            .track
            .cover
            .clone()
            .and_then(|path| self.covers.load(path));
        let track = to_track_data(&loaded.track, index as i32, cover);
        self.changes.push(Changes::Metadata(self.row(index), track));
        self.loaded += 1;
        self.progress_dirty = true;
        self.apply_ui();
    }
}

impl Handle<ScanFinished> for AppScene {
    fn handle(&mut self, _msg: &ScanFinished, _ctx: &Ctx, _out: &mut Outbox) {
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
