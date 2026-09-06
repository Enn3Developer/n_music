mod library;
mod playback_mirror;

use crate::jobs::scan::ScanJob;
use crate::messages::{
    PlaybackChanged, PositionChanged, ScanLibrary, SearchChanged, TrackChanged, VolumeChanged,
};
use crate::{AppData, MainWindow, TrackData};
use n_event_bus::{Ctx, Registrar, RunningJob, Scene, Subscriber, UiPatch};
use slint::{ComponentHandle, Model, VecModel, Weak};
use std::any::Any;
use std::mem;

pub enum Changes {
    Tracks(Vec<TrackData>),
    Metadata(usize, TrackData),
}

pub struct AppScene {
    window: Weak<MainWindow>,
    scan_job: Option<RunningJob>,
    playing_index: i32,
    position: f64,
    seek_revision: i32,
    position_str: String,
    length: f64,
    playback: bool,
    volume: f64,
    track_count: usize,
    loaded: usize,
    changes: Vec<Changes>,
    search: String,
    search_dirty: bool,
    save_y: bool,
    progress_dirty: bool,
    dirty: bool,
    visible: bool,
}

impl AppScene {
    pub fn new(window: Weak<MainWindow>, volume: f64) -> Self {
        Self {
            window,
            scan_job: None,
            playing_index: 0,
            position: 0.0,
            seek_revision: 0,
            position_str: String::from("00:00"),
            length: 0.0,
            playback: false,
            volume,
            track_count: 0,
            loaded: 0,
            changes: vec![],
            search: String::new(),
            search_dirty: false,
            save_y: false,
            progress_dirty: false,
            dirty: true,
            visible: true,
        }
    }
}

impl Subscriber for AppScene {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<PlaybackChanged>();
        reg.on::<TrackChanged>();
        reg.on::<VolumeChanged>();
        reg.on::<PositionChanged>();
        reg.on::<ScanLibrary>();
        reg.on::<crate::messages::Shutdown>();
        reg.on::<crate::messages::AppVisibilityChanged>();
        reg.on::<SearchChanged>();
        ScanJob::subscribe(reg);
    }
}

impl Scene for AppScene {
    fn sync(&mut self, _ctx: &Ctx) -> Option<UiPatch> {
        if !self.visible
            || !(self.dirty || self.progress_dirty || self.search_dirty || !self.changes.is_empty())
        {
            return None;
        }
        self.dirty = false;
        let playing = self.playing_index;
        let position = self.position;
        let seek_revision = self.seek_revision;
        let position_str = self.position_str.clone();
        let length = self.length;
        let playback = self.playback;
        let volume = self.volume;

        let changes = mem::take(&mut self.changes);
        let updated_search = mem::take(&mut self.search_dirty);
        let save_y = mem::take(&mut self.save_y);
        let new_loaded = !changes.is_empty() || mem::take(&mut self.progress_dirty);
        let progress = if self.track_count > 0 {
            self.loaded as f64 / self.track_count as f64
        } else {
            0.0
        };
        let search = self.search.to_lowercase();

        let window = self.window.clone();
        Some(Box::new(move || {
            let Some(window) = window.upgrade() else {
                return;
            };
            let app_data = window.global::<AppData>();
            app_data.set_playing(playing);
            app_data.set_position_time(position_str.into());
            if !app_data.get_seeking() && seek_revision == app_data.get_seek_revision() {
                app_data.set_time(position as f32);
            }
            app_data.set_length(length as f32);
            app_data.set_playback(playback);
            app_data.set_volume(volume as f32);

            if new_loaded {
                let progress = if progress == 1.0 {
                    0.0
                } else {
                    progress as f32
                };
                app_data.set_progress(progress);
            }

            for change in changes {
                match change {
                    Changes::Tracks(tracks) => {
                        app_data.set_tracks(VecModel::from_slice(&tracks));
                    }
                    Changes::Metadata(index, track) => {
                        app_data.get_tracks().set_row_data(index, track);
                    }
                }
            }

            if updated_search || new_loaded {
                let tracks = app_data.get_tracks();
                let mut counter = 0;
                if save_y {
                    app_data.set_saved_y(app_data.get_viewport_y());
                }
                for (index, mut track) in tracks.iter().enumerate() {
                    let title = track.title.to_lowercase();
                    let artist = track.artist.to_lowercase();
                    if search.is_empty() && !track.visible {
                        track.visible = true;
                    } else if !search.is_empty() {
                        if title.contains(&search) || artist.contains(&search) {
                            counter += 1;
                            if track.visible {
                                continue;
                            }
                            track.visible = true;
                        } else {
                            if !track.visible {
                                continue;
                            }
                            track.visible = false;
                        }
                    } else {
                        continue;
                    }
                    tracks.set_row_data(index, track);
                }
                let height = (counter * -84) as f32;
                if height > app_data.get_viewport_y() {
                    app_data.set_viewport_y(0.0);
                }
                if search.is_empty() {
                    app_data.set_viewport_y(app_data.get_saved_y());
                }
            }
        }))
    }
}

impl n_event_bus::Handle<crate::messages::Shutdown> for AppScene {
    fn handle(&mut self, _: &crate::messages::Shutdown, _: &Ctx, _: &mut n_event_bus::Outbox) {
        self.scan_job = None;
        self.visible = false;
    }
}
impl n_event_bus::Handle<crate::messages::AppVisibilityChanged> for AppScene {
    fn handle(
        &mut self,
        msg: &crate::messages::AppVisibilityChanged,
        _: &Ctx,
        _: &mut n_event_bus::Outbox,
    ) {
        self.visible = msg.0;
        self.dirty = true;
    }
}
