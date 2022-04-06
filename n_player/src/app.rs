use std::collections::HashMap;
use std::fs;
use std::fs::DirEntry;

use eframe::egui::{
    Button, Label, Response, ScrollArea, Slider, SliderOrientation, Visuals, Widget,
};
use eframe::{egui, epi};
use itertools::Itertools;

use n_audio::player::Player;
use n_audio::{from_path_to_name_without_ext, TrackTime};

use crate::Config;

pub struct App {
    config: Config,
    path: String,
    player: Player,
    volume: f32,
    time: f64,
    cached_track_time: Option<TrackTime>,
    files: HashMap<String, u64>,
}

impl App {
    pub fn new(config: Config, config_path: String, player: Player) -> Self {
        let path = config.path().clone().unwrap();
        let paths = fs::read_dir(path).expect("Can't read files in the chosen directory");
        let entries: Vec<DirEntry> = paths
            .filter(|item| item.is_ok())
            .map(|item| item.unwrap())
            .collect();
        let mut files = HashMap::new();

        for entry in &entries {
            if entry.metadata().unwrap().is_file()
                && infer::get_from_path(entry.path())
                    .unwrap()
                    .unwrap()
                    .mime_type()
                    .contains("audio")
            {
                let name = from_path_to_name_without_ext(&entry.path());
                let duration =
                    player.get_duration_for_track(player.get_index_from_track_name(&name).unwrap());
                files.insert(name, duration.dur_secs);
            }
        }

        let path = config_path;

        let volume = config.volume_or_default(1.0) as f32;

        Self {
            config,
            path,
            player,
            volume,
            time: 0.0,
            cached_track_time: None,
            files,
        }
    }

    fn slider_seek(&mut self, slider: Response, track_time: Option<TrackTime>) {
        if let Some(track_time) = track_time {
            if slider.drag_released() || slider.clicked() {
                self.player.pause().unwrap();
                let total_time = track_time.dur_secs as f64 + track_time.dur_frac;
                let seek_time = total_time * self.time;
                self.player
                    .seek_to(seek_time.floor() as u64, seek_time.fract())
                    .unwrap();
                self.player.unpause().unwrap();
            }
        }
    }
}

impl epi::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        ctx.set_visuals(Visuals::dark());

        if self.player.has_ended() {
            self.player.play_next();
        }

        self.player.set_volume(self.volume).unwrap();

        egui::TopBottomPanel::bottom("control_panel").show(ctx, |ui| {
            ui.set_min_height(40.0);

            let track_time = self.player.get_time();
            self.time = if let Some(track_time) = &track_time {
                let value = (track_time.ts_secs as f64 + track_time.ts_frac)
                    / (track_time.dur_secs as f64 + track_time.dur_frac);
                self.cached_track_time = Some(track_time.clone());
                value
            } else if let Some(track_time) = &self.cached_track_time {
                (track_time.ts_secs as f64 + track_time.ts_frac)
                    / (track_time.dur_secs as f64 + track_time.dur_frac)
            } else {
                0.0
            };

            ui.horizontal(|ui| {
                let slider = Slider::new(&mut self.time, 0.0..=1.0)
                    .orientation(SliderOrientation::Horizontal)
                    .show_value(false)
                    .ui(ui);
                ui.add_space(10.0);

                let volume_slider = Slider::new(&mut self.volume, 0.0..=1.0)
                    .show_value(false)
                    .ui(ui);

                self.slider_seek(slider, track_time);

                if volume_slider.changed() {
                    self.config.set_volume(self.volume as f64);
                    self.config.save(&self.path).unwrap();
                }
            });

            ui.horizontal(|ui| {
                ScrollArea::horizontal().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    ui.label(self.player.get_current_track_name());
                    ui.add_space(10.0);

                    let text_toggle = if !self.player.is_playing() || self.player.is_paused() {
                        "▶"
                    } else {
                        "⏸"
                    };

                    let previous = Button::new("⏮").frame(false).ui(ui);
                    let toggle = Button::new(text_toggle).frame(false).ui(ui);
                    let next = Button::new("⏭").frame(false).ui(ui);

                    if previous.clicked() {
                        if let Some(cached_track_time) = &self.cached_track_time {
                            if cached_track_time.ts_secs < 2 {
                                self.player.seek_to(0, 0.0).unwrap();
                            }
                            self.player.play_previous();
                        }
                    }
                    if toggle.clicked() {
                        if self.player.is_paused() {
                            self.player.unpause().unwrap();
                        } else {
                            self.player.pause().unwrap();
                        }
                        if !self.player.is_playing() {
                            self.player.play_next();
                        }
                    }
                    if next.clicked() {
                        self.player.play_next();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::warn_if_debug_build(ui);
            ScrollArea::vertical().show(ui, |ui| {
                for (name, duration) in self.files.iter().sorted() {
                    ui.horizontal(|ui| {
                        let mut frame = false;
                        if self.player.is_playing() && &self.player.get_current_track_name() == name
                        {
                            ui.add_space(10.0);
                            frame = true;
                        }
                        let button = Button::new(name).frame(frame).ui(ui);
                        ui.add(Label::new(format!(
                            "{:02}:{:02}",
                            duration / 60,
                            duration % 60
                        )));

                        if button.clicked() {
                            let index = self.player.get_index_from_track_name(name).unwrap();
                            self.player.end_current().unwrap();
                            self.player.play(index, true);
                        }
                    });
                }
                ui.allocate_space(ui.available_size());
            });
        });

        // self.title = format!("N Music - {}", self.player.get_current_track_name());

        ctx.request_repaint();
    }

    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
    }

    fn on_exit_event(&mut self) -> bool {
        self.player.end_current().unwrap();
        self.config.save(&self.path).unwrap();
        true
    }

    fn name(&self) -> &str {
        "N Music"
    }
}
