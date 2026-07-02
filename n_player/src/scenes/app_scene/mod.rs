mod library;
mod playback_mirror;

use crate::jobs::scan::ScanJob;
use crate::messages::{
    PlaybackChanged, PositionChanged, ScanRequested, SearchChanged, TrackChanged,
    ViewportChanging, VolumeChanged,
};
use crate::platform::Platform;
use crate::settings::Settings;
use crate::{AppData, MainWindow, TrackData};
use n_event_bus::{Ctx, Registrar, RunningJob, Scene, Subscriber, UiPatch};
use slint::{ComponentHandle, Model, VecModel, Weak};
use std::any::Any;
use std::mem;
use std::sync::Arc;
use tokio::sync::RwLock;

pub enum Changes {
    Tracks(Vec<TrackData>),
    Metadata(usize, TrackData),
}

/// Mirrors ui/scenes/app.slint's `App` component: the track list + search
/// (library half) and a display copy of PlaybackEngine's state (playback half).
pub struct AppScene {
    window: Weak<MainWindow>,
    settings: Arc<RwLock<Settings>>,
    platform: Arc<dyn Platform>,
    scan_job: Option<RunningJob>,
    // Playback mirror.
    playing_index: i32,
    position: f64,
    position_str: String,
    length: f64,
    playback: bool,
    volume: f64,
    skip_time: bool,
    // Library.
    track_count: usize,
    loaded: usize,
    changes: Vec<Changes>,
    search: String,
    search_dirty: bool,
    save_y: bool,
    progress_dirty: bool,
}

impl AppScene {
    pub fn new(
        window: Weak<MainWindow>,
        settings: Arc<RwLock<Settings>>,
        platform: Arc<dyn Platform>,
    ) -> Self {
        Self {
            window,
            settings,
            platform,
            scan_job: None,
            playing_index: 0,
            position: 0.0,
            position_str: String::from("00:00"),
            length: 0.0,
            playback: false,
            volume: 1.0,
            skip_time: false,
            track_count: 0,
            loaded: 0,
            changes: vec![],
            search: String::new(),
            search_dirty: false,
            save_y: false,
            progress_dirty: false,
        }
    }
}

impl Subscriber for AppScene {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        // Playback mirror.
        reg.on::<PlaybackChanged>();
        reg.on::<TrackChanged>();
        reg.on::<VolumeChanged>();
        reg.on::<PositionChanged>();
        reg.on::<ViewportChanging>();
        // Library.
        reg.on::<ScanRequested>();
        reg.on::<SearchChanged>();
        ScanJob::subscribe(reg);
    }
}

impl Scene for AppScene {
    fn sync(&mut self, _ctx: &Ctx) -> Option<UiPatch> {
        let playing = self.playing_index;
        let position = self.position;
        let position_str = self.position_str.clone();
        let length = self.length;
        let playback = self.playback;
        let volume = self.volume;
        let skip_time = mem::take(&mut self.skip_time);

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
            if !skip_time {
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
