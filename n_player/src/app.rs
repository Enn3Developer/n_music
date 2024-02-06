use std::collections::HashMap;
use std::fs;
use std::fs::DirEntry;
#[cfg(windows)]
use std::path::PathBuf;

use eframe::egui;
use eframe::egui::{
    Button, Label, Response, ScrollArea, Slider, SliderOrientation, ViewportCommand, Visuals,
    Widget,
};
use eframe::epaint::FontFamily;
use eframe::glow::Context;
use itertools::Itertools;

use n_audio::queue::QueuePlayer;
use n_audio::{from_path_to_name_without_ext, TrackTime};

use crate::Config;

pub struct App {
    config: Config,
    path: String,
    player: QueuePlayer,
    volume: f32,
    time: f64,
    cached_track_time: Option<TrackTime>,
    files: HashMap<String, u64>,
    title: String,
}

impl App {
    pub fn new(
        config: Config,
        config_path: String,
        player: QueuePlayer,
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
        Self::configure_fonts(&cc.egui_ctx);

        let path = config.path().clone().unwrap();
        let paths = fs::read_dir(path).expect("Can't read files in the chosen directory");
        let entries: Vec<DirEntry> = paths.filter_map(|item| item.ok()).collect();
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
            title: String::from("N Music"),
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

    pub fn configure_fonts(ctx: &egui::Context) -> Option<()> {
        let font_file = Self::find_cjk_font()?;
        let font_name = font_file.split('/').last()?.split('.').next()?.to_string();
        let font_file_bytes = fs::read(font_file).ok()?;

        let font_data = egui::FontData::from_owned(font_file_bytes);
        let mut font_def = eframe::egui::FontDefinitions::default();
        font_def.font_data.insert(font_name.to_string(), font_data);

        font_def
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .push(font_name);

        ctx.set_fonts(font_def);
        Some(())
    }

    fn find_cjk_font() -> Option<String> {
        #[cfg(unix)]
        {
            use std::process::Command;
            // linux/macOS command: fc-list
            let output = Command::new("sh").arg("-c").arg("fc-list").output().ok()?;
            let stdout = std::str::from_utf8(&output.stdout).ok()?;
            #[cfg(target_os = "macos")]
            let font_line = stdout
                .lines()
                .find(|line| line.contains("Regular") && line.contains("Hiragino Sans GB"))
                .unwrap_or("/System/Library/Fonts/Hiragino Sans GB.ttc");
            #[cfg(target_os = "linux")]
            let font_line = stdout
                .lines()
                .find(|line| line.contains("Regular") && line.contains("CJK"))
                .unwrap_or("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc");

            let font_path = font_line.split(':').next()?.trim();
            Some(font_path.to_string())
        }
        #[cfg(windows)]
        {
            let font_file = {
                // c:/Windows/Fonts/msyh.ttc
                let mut font_path = PathBuf::from(std::env::var("SystemRoot").ok()?);
                font_path.push("Fonts");
                font_path.push("msyh.ttc");
                font_path.to_str()?.to_string().replace("\\", "/")
            };
            Some(font_file)
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

                    ui.label(self.player.current_track_name());
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
                            } else {
                                self.player.end_current().unwrap();
                                self.player.play_previous();
                            }
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
                        self.player.end_current().unwrap();
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
                        if self.player.is_playing() && self.player.current_track_name() == name {
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
                            self.player.play(index);
                        }
                    });
                }
                ui.allocate_space(ui.available_size());
            });
        });

        self.title = format!("N Music - {}", self.player.current_track_name());
        ctx.send_viewport_cmd(ViewportCommand::Title(self.title.clone()));

        ctx.request_repaint();
    }

    fn on_exit(&mut self, _gl: Option<&Context>) {
        self.player.end_current().unwrap();
        self.config.save(&self.path).unwrap();
    }
}
