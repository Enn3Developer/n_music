use crate::runner::{Runner, RunnerMessage, Seek};
use crate::{loader_thread, FileTrack, FileTracks, Message};
use eframe::egui::{
    Button, Context, Event, Key, Modifiers, ScrollArea, Slider, SliderOrientation, Visuals, Widget,
};
use eframe::{egui, CreationContext, Frame};
use flume::{Receiver, Sender};
use n_audio::{remove_ext, TrackTime};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::RwLock;

pub struct App {
    runner: Arc<RwLock<Runner>>,
    tx: Sender<RunnerMessage>,
    rx: Receiver<Message>,
    len: usize,
    playback: bool,
    volume: f64,
    time: TrackTime,
    tracks: FileTracks,
    slider_time: f64,
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
                FileTrack::new(remove_ext(track_path), String::new(), 0)
            })
            .collect::<Vec<_>>()
            .into();

        let queue = runner.blocking_read().queue();
        let path = runner.blocking_read().path();
        let (tx_l, rx) = flume::unbounded();
        thread::spawn(move || {
            let paths = queue
                .into_iter()
                .map(|file_name| {
                    let mut path_buf = PathBuf::new();
                    path_buf.push(&path);
                    path_buf.push(file_name);
                    path_buf.to_str().unwrap().to_string()
                })
                .collect::<Vec<_>>();
            loader_thread(tx_l, paths);
        });

        Self {
            runner,
            tx,
            rx,
            len,
            playback: false,
            volume: 1.0,
            time: TrackTime::default(),
            tracks,
            slider_time: 0.0,
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

    pub fn seek(&self) {
        if self.time.len_secs == 0 {
            return;
        }
        self.tx
            .send(RunnerMessage::Seek(Seek::Absolute(
                self.slider_time * (self.time.len_secs as f64 + self.time.len_frac),
            )))
            .unwrap();
    }

    pub fn play_track(&self, i: usize) {
        self.tx.send(RunnerMessage::PlayTrack(i)).unwrap();
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
        self.slider_time = (self.time.pos_secs as f64 + self.time.pos_frac)
            / (self.time.len_secs as f64 + self.time.len_frac);

        while let Ok(message) = self.rx.try_recv() {
            match message {
                Message::Length(index, length) => self.tracks[index].length = length,
                Message::Artist(index, artist) => self.tracks[index].artist = artist,
                Message::Title(index, title) => self.tracks[index].title = title,
            }
        }

        ctx.input(|input| {
            for event in &input.events {
                match event {
                    Event::Key {
                        key: Key::Space,
                        pressed: true,
                        repeat: false,
                        ..
                    } => self.toggle_pause(),
                    Event::Key {
                        key: Key::ArrowRight,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers { ctrl: true, .. },
                        ..
                    } => self.play_next(),
                    Event::Key {
                        key: Key::ArrowLeft,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers { ctrl: true, .. },
                        ..
                    } => self.play_previous(),
                    _ => {}
                };
            }
        });

        egui::TopBottomPanel::bottom("control_panel").show(ctx, |ui| {
            ui.set_min_height(60.0);
            let mut index = self.runner.blocking_read().index();
            if index > self.tracks.len() {
                index = 0;
            }
            ui.horizontal(|ui| {
                ui.label(self.time.format_pos());
                let time_slider = Slider::new(&mut self.slider_time, 0.0..=1.0)
                    .orientation(SliderOrientation::Horizontal)
                    .show_value(false)
                    .ui(ui);
                ui.label(format!(
                    "{:02}:{:02}",
                    (self.tracks[index].length as f64 / 60.0).floor() as u64,
                    self.tracks[index].length % 60
                ));
                ui.add_space(10.0);
                let volume_slider = Slider::new(&mut self.volume, 0.0..=1.0)
                    .show_value(false)
                    .ui(ui);
                ui.label(format!("{}%", (self.volume * 100.0).round() as usize));

                if time_slider.changed() {
                    self.seek();
                }
                if volume_slider.changed() {
                    self.set_volume();
                }
            });
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

    fn persist_egui_memory(&self) -> bool {
        false
    }
}
