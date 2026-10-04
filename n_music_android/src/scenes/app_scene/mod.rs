mod covers;
mod library;
mod playback;

use crate::messages::SearchChanged;
use crate::ui::{CoverBuffer, CoverCache};
use crate::{AppData, MainWindow, TrackData};
use n_event_bus::{Ctx, Message, Registrar, RunningJob, ShutdownRequested, Subscriber};
use n_music_core::messages::{
    LoopStatusChanged, PlaybackChanged, PositionChanged, QueueChanged, ScanFinished, ScanRequested,
    TrackChanged, TrackMetadataLoaded, TracksEnumerated, VolumeChanged,
};
use n_music_core::queue::LoopStatus;
use n_music_core::source::Locator;
use slint::{ComponentHandle, Model, VecModel, Weak};
use std::any::Any;
use std::collections::HashMap;
use std::mem;

/// A row of the track list was clicked; the library index of its track.
pub struct TrackClicked(pub usize);

impl Message for TrackClicked {}

pub enum Changes {
    Tracks(Vec<TrackData>),
    /// Rows rearranged into this order of library indexes.
    Order(Vec<usize>),
    Metadata(usize, TrackData),
    Cover(usize, CoverBuffer),
}

pub struct AppScene {
    window: Weak<MainWindow>,
    cover_job: Option<RunningJob>,
    /// Row of the current track.
    playing_index: i32,
    /// The current track.
    playing: Option<Locator>,
    /// The library's tracks, by library index.
    locators: Vec<Locator>,
    index_of: HashMap<Locator, usize>,
    /// Row of each library index; rows follow the play order.
    row_of: Vec<usize>,
    position: f64,
    seek_revision: i32,
    position_str: String,
    length: f64,
    playback: bool,
    loop_status: LoopStatus,
    volume: f64,
    track_count: usize,
    loaded: usize,
    changes: Vec<Changes>,
    search: String,
    search_dirty: bool,
    save_y: bool,
    progress_dirty: bool,
    dirty: bool,
    position_dirty: bool,
    position_text_dirty: bool,
    length_dirty: bool,
    visible: bool,
    covers: CoverCache,
}

impl AppScene {
    pub fn new(window: Weak<MainWindow>) -> Self {
        Self {
            window,
            cover_job: None,
            playing_index: 0,
            playing: None,
            locators: vec![],
            index_of: HashMap::new(),
            row_of: vec![],
            position: 0.0,
            seek_revision: 0,
            position_str: String::from("00:00"),
            length: 0.0,
            playback: false,
            loop_status: LoopStatus::default(),
            volume: 1.0,
            track_count: 0,
            loaded: 0,
            changes: vec![],
            search: String::new(),
            search_dirty: false,
            save_y: false,
            progress_dirty: false,
            dirty: true,
            position_dirty: true,
            position_text_dirty: true,
            length_dirty: true,
            visible: true,
            covers: CoverCache::default(),
        }
    }
}

impl Subscriber for AppScene {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<PlaybackChanged>();
        reg.on::<LoopStatusChanged>();
        reg.on::<TrackChanged>();
        reg.on::<VolumeChanged>();
        reg.on::<PositionChanged>();
        reg.on::<ScanRequested>();
        reg.on::<ShutdownRequested>();
        reg.on::<n_music_core::messages::AppVisibilityChanged>();
        reg.on::<SearchChanged>();
        reg.on::<QueueChanged>();
        reg.on::<TracksEnumerated>();
        reg.on::<TrackMetadataLoaded>();
        reg.on::<ScanFinished>();
        reg.on::<TrackClicked>();
        covers::CoverJob::subscribe(reg);
    }
}

impl AppScene {
    /// The row showing the track at library `index`.
    pub(crate) fn row(&self, index: usize) -> usize {
        self.row_of.get(index).copied().unwrap_or(index)
    }

    /// The row of the current track, or 0.
    pub(crate) fn playing_row(&self) -> i32 {
        self.playing
            .as_ref()
            .and_then(|locator| self.index_of.get(locator))
            .map_or(0, |&index| self.row(index) as i32)
    }

    pub(crate) fn apply_ui(&mut self) {
        if !self.visible
            || !(self.dirty
                || self.position_dirty
                || self.progress_dirty
                || self.search_dirty
                || !self.changes.is_empty())
        {
            return;
        }
        let state_dirty = mem::take(&mut self.dirty);
        let position_dirty = mem::take(&mut self.position_dirty) || state_dirty;
        let position_text_dirty = mem::take(&mut self.position_text_dirty) || state_dirty;
        let length_dirty = mem::take(&mut self.length_dirty) || state_dirty;
        let playing = self.playing_index;
        let position = self.position;
        let seek_revision = self.seek_revision;
        let position_str = position_text_dirty.then(|| self.position_str.clone());
        let length = self.length;
        let playback = self.playback;
        let repeat_one = self.loop_status == LoopStatus::File;
        let volume = self.volume;

        let changes = mem::take(&mut self.changes);
        let updated_search = mem::take(&mut self.search_dirty);
        let save_y = mem::take(&mut self.save_y);
        let progress_dirty = mem::take(&mut self.progress_dirty);
        let new_loaded = !changes.is_empty() || progress_dirty;
        let progress = if self.track_count > 0 {
            self.loaded as f64 / self.track_count as f64
        } else {
            0.0
        };
        let search = (updated_search || new_loaded).then(|| self.search.to_lowercase());

        let window = self.window.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(window) = window.upgrade() else {
                return;
            };
            let app_data = window.global::<AppData>();
            if state_dirty {
                app_data.set_playing(playing);
                app_data.set_playback(playback);
                app_data.set_repeat_one(repeat_one);
                app_data.set_volume(volume as f32);
            }
            if let Some(position_str) = position_str {
                app_data.set_position_time(position_str.into());
            }
            if position_dirty
                && !app_data.get_seeking()
                && seek_revision == app_data.get_seek_revision()
            {
                app_data.set_time(position as f32);
            }
            if length_dirty {
                app_data.set_length(length as f32);
            }

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
                    Changes::Order(order) => {
                        let tracks = app_data.get_tracks();
                        let mut by_index: Vec<Option<TrackData>> = vec![None; tracks.row_count()];
                        for track in tracks.iter() {
                            if let Some(slot) = by_index.get_mut(track.index as usize) {
                                *slot = Some(track);
                            }
                        }
                        for (row, index) in order.into_iter().enumerate() {
                            if let Some(track) = by_index.get_mut(index).and_then(Option::take) {
                                tracks.set_row_data(row, track);
                            }
                        }
                    }
                    Changes::Metadata(index, track) => {
                        app_data.get_tracks().set_row_data(index, track);
                    }
                    Changes::Cover(index, buffer) => {
                        let tracks = app_data.get_tracks();
                        if let Some(mut track) = tracks.row_data(index) {
                            track.cover = slint::Image::from_rgba8(buffer);
                            tracks.set_row_data(index, track);
                        }
                    }
                }
            }

            if let Some(search) = search {
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
        });
    }
}

impl n_event_bus::Handle<ShutdownRequested> for AppScene {
    fn handle(&mut self, _: &ShutdownRequested, _: &Ctx, out: &mut n_event_bus::Outbox) {
        self.cover_job = None;
        self.visible = false;
        out.shutdown_ready();
    }
}
impl n_event_bus::Handle<n_music_core::messages::AppVisibilityChanged> for AppScene {
    fn handle(
        &mut self,
        msg: &n_music_core::messages::AppVisibilityChanged,
        ctx: &Ctx,
        _: &mut n_event_bus::Outbox,
    ) {
        if ctx.shutting_down {
            return;
        }
        self.visible = msg.0;
        self.dirty = true;
        self.apply_ui();
    }
}
