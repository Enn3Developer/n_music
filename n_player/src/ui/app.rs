use crate::runner::{Runner, RunnerMessage};
use crate::{FileTrack, FileTracks};
use eframe::egui::{Button, Context, ScrollArea, Visuals, Widget};
use eframe::{egui, CreationContext, Frame};
use flume::Sender;
use n_audio::music_track::MusicTrack;
use n_audio::{remove_ext, TrackTime};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub struct App {
    runner: Arc<RwLock<Runner>>,
    tx: Sender<RunnerMessage>,
    len: usize,
    playback: bool,
    volume: f64,
    time: TrackTime,
    tracks: FileTracks,
}

impl App {
    pub fn new(
        runner: Arc<RwLock<Runner>>,
        tx: Sender<RunnerMessage>,
        _cc: &CreationContext,
    ) -> Self {
        let len = runner.blocking_read().len();
        let tracks: FileTracks = (0..len)
            .into_iter()
            .map(|i| {
                let track_path = runner.blocking_read().get_path_for_file(i);
                let track = MusicTrack::new(track_path.to_string_lossy().to_string()).unwrap();
                let metadata = track.get_meta();
                FileTrack::new(
                    if !metadata.title.is_empty() {
                        metadata.title
                    } else {
                        remove_ext(track_path)
                    },
                    metadata.artist,
                    metadata.time.len_secs,
                )
            })
            .collect::<Vec<_>>()
            .into();

        Self {
            runner,
            tx,
            len,
            playback: false,
            volume: 1.0,
            time: TrackTime::default(),
            tracks,
        }
    }

    pub fn play_next(&self) {
        self.tx.send(RunnerMessage::PlayNext).unwrap();
    }

    pub fn play_previous(&self) {
        self.tx.send(RunnerMessage::PlayPrevious).unwrap();
    }

    pub fn toggle_pause(&self) {
        self.tx.send(RunnerMessage::TogglePause).unwrap();
    }

    pub fn set_volume(&self) {
        self.tx.send(RunnerMessage::SetVolume(self.volume)).unwrap();
    }

    pub fn play_track(&self, i: usize) {
        self.tx.send(RunnerMessage::PlayTrack(i)).unwrap()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        ctx.set_visuals(Visuals::dark());

        {
            let guard = self.runner.blocking_read();
            self.playback = guard.playback();
            self.volume = guard.volume();
            self.time = guard.time();
        }

        ctx.input(|input| {});

        egui::TopBottomPanel::bottom("control_panel").show(ctx, |ui| {
            ui.set_min_height(60.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::warn_if_debug_build(ui);
            let row_height = 40.0;
            ScrollArea::vertical().show_rows(ui, row_height, self.len, |ui, rows| {
                for row in rows {
                    let track = &self.tracks[row];
                    let title = &track.title;
                    let artist = &track.artist;
                    ui.horizontal(|ui| {
                        let mut frame = false;
                        if self.playback && self.runner.blocking_read().index() == row {
                            ui.add_space(10.0);
                            frame = true;
                        }
                        ui.vertical(|ui| {
                            let button = Button::new(title).frame(frame).ui(ui);
                            ui.label(artist);

                            if button.clicked() {
                                self.play_track(row);
                            }
                        });
                        ui.label(format!(
                            "{:02}:{:02}",
                            (track.length as f64 / 60.0).floor() as u64,
                            track.length % 60
                        ))
                    });
                    if row + 1 != self.len {
                        ui.separator();
                    }
                }
            });
        });

        ctx.request_repaint_after(Duration::from_millis(250));
    }
}
