use crate::{
    add_all_tracks_to_player, loader_thread, vec_contains, FileTrack, FileTracks, Message,
};
use eframe::egui::{
    Button, Event, Key, Label, Modifiers, Response, ScrollArea, Slider, SliderOrientation,
    ViewportCommand, Visuals, Widget,
};
use eframe::epaint::FontFamily;
use eframe::{egui, Storage};
use flume::Receiver;
#[cfg(target_os = "linux")]
use mpris_server::RootInterface;
use mpris_server::{
    LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, Time, TrackId, Volume,
};
use n_audio::queue::QueuePlayer;
use n_audio::{from_path_to_name_without_ext, TrackTime};
use std::fs::DirEntry;
#[cfg(target_os = "windows")]
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::{fs, thread};

pub struct App {
    path: Option<String>,
    player: QueuePlayer,
    volume: f32,
    time: f64,
    cached_track_time: Option<TrackTime>,
    files: FileTracks,
    rx: Option<Receiver<Message>>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::configure_fonts(&cc.egui_ctx);
        let mut player = QueuePlayer::new(String::new());
        let mut files = FileTracks { tracks: vec![] };
        let mut saved_files = FileTracks { tracks: vec![] };
        let mut volume = 1.0;
        let mut maybe_path = None;
        if let Some(storage) = cc.storage {
            if let Some(data) = storage.get_string("track_files") {
                if let Ok(read_data) = toml::from_str(&data) {
                    saved_files = read_data;
                }
            }
            if let Some(data_v) = storage.get_string("volume") {
                volume = data_v.parse().unwrap_or(1.0);
            }
            if let Some(path) = storage.get_string("path") {
                player.set_path(path.clone());
                add_all_tracks_to_player(&mut player, path.clone());
                maybe_path = Some(path);
            }
        }
        player.set_volume(volume).unwrap();
        let mut rx = None;
        if let Some(path) = &maybe_path {
            rx = Some(Self::init(
                PathBuf::new().join(path),
                &mut player,
                &mut files,
                &saved_files,
            ));
        }
        Self {
            path: maybe_path,
            player,
            volume,
            time: 0.0,
            cached_track_time: None,
            files,
            rx,
        }
    }

    fn slider_seek(&mut self, slider: Response, track_time: Option<TrackTime>) {
        if let Some(track_time) = track_time {
            if slider.changed() {
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
        let mut font_def = egui::FontDefinitions::default();
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

    fn finish_init(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Add music folder");
            if ui.button("Open folder…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    let saved = FileTracks { tracks: vec![] };
                    ui.label("Loading...");
                    let rx = Self::init(path.clone(), &mut self.player, &mut self.files, &saved);
                    self.path = Some(path.to_str().unwrap().to_string());
                    self.rx = Some(rx);
                }
            }
        });
    }

    fn init(
        path: PathBuf,
        player: &mut QueuePlayer,
        files: &mut FileTracks,
        saved_files: &FileTracks,
    ) -> Receiver<Message> {
        let paths = fs::read_dir(&path).expect("Can't read files in the chosen directory");
        let entries: Vec<DirEntry> = paths.filter_map(|item| item.ok()).collect();
        let mut indexing_files = Vec::with_capacity(entries.len());
        add_all_tracks_to_player(player, path.to_str().unwrap().to_string());
        for entry in &entries {
            if entry.metadata().unwrap().is_file()
                && infer::get_from_path(entry.path())
                    .unwrap()
                    .unwrap()
                    .mime_type()
                    .contains("audio")
            {
                let mut name = from_path_to_name_without_ext(&entry.path());
                name.shrink_to_fit();
                let contains = vec_contains(saved_files, &name);
                let (duration, mut artist, cover) = if contains.0 {
                    (
                        saved_files[contains.1].duration,
                        saved_files[contains.1].artist.clone(),
                        saved_files[contains.1].cover.clone(),
                    )
                } else {
                    (0, "ARTIST".to_string(), vec![])
                };
                artist.shrink_to_fit();
                files.push(FileTrack {
                    name,
                    duration,
                    artist,
                    cover,
                });
                indexing_files.push(entry.path());
            }
        }
        files.sort();
        indexing_files.sort();
        let (tx, rx) = flume::unbounded();
        thread::spawn(|| loader_thread(tx, indexing_files));
        rx
    }

    fn update_title(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(ViewportCommand::Title(format!(
            "N Music - {}",
            self.player.current_track_name().rsplit_once('.').unwrap().0
        )));
    }

    fn toggle_pause(&mut self, ctx: &egui::Context) {
        if self.player.is_paused() {
            self.player.unpause().unwrap();
        } else {
            self.player.pause().unwrap();
        }
        if !self.player.is_playing() {
            self.player.play_next();
            self.update_title(ctx);
        }
    }

    fn play_next(&mut self, ctx: &egui::Context) {
        self.player.end_current().unwrap();
        self.player.play_next();
        self.update_title(ctx);
    }

    fn play_previous(&mut self, ctx: &egui::Context) {
        if let Some(cached_track_time) = &self.cached_track_time {
            if cached_track_time.ts_secs < 2 {
                self.player.seek_to(0, 0.0).unwrap();
            } else {
                self.player.end_current().unwrap();
                self.player.play_previous();
                self.update_title(ctx);
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(Visuals::dark());
        if self.path.is_none() {
            self.finish_init(ctx);
            return;
        }
        if let Some(rx) = &self.rx {
            while let Ok(message) = rx.try_recv() {
                match message {
                    Message::Duration(i, dur) => {
                        self.files[i].duration = dur;
                    }
                    Message::Artist(i, artist) => {
                        self.files[i].artist = artist;
                    } // Message::Image(i, data) => {
                      //     self.files[i].cover = data;
                      // }
                }
            }
        }
        if self.player.has_ended() {
            self.player.play_next();
            self.update_title(ctx);
        }
        let mut pause = false;
        let mut next = false;
        let mut previous = false;
        ctx.input(|i| {
            for event in &i.events {
                match event {
                    Event::Key {
                        key: Key::Space,
                        pressed: true,
                        repeat: false,
                        ..
                    } => pause = true,
                    Event::Key {
                        key: Key::ArrowRight,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers { ctrl: true, .. },
                        ..
                    } => next = true,
                    Event::Key {
                        key: Key::ArrowLeft,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers { ctrl: true, .. },
                        ..
                    } => previous = true,
                    _ => {}
                };
            }
        });
        egui::TopBottomPanel::bottom("control_panel").show(ctx, |ui| {
            ui.set_min_height(40.0);
            let track_time = self.player.get_time();
            let current_time: String;
            let total_time: String;
            self.time = if let Some(track_time) = &track_time {
                let value = (track_time.ts_secs as f64 + track_time.ts_frac)
                    / (track_time.dur_secs as f64 + track_time.dur_frac);
                self.cached_track_time = Some(track_time.clone());
                current_time = format!(
                    "{:02}:{:02}",
                    ((track_time.ts_secs as f64 + track_time.ts_frac) / 60.0).round() as u64,
                    track_time.ts_secs % 60
                );
                total_time = format!(
                    "{:02}:{:02}",
                    ((track_time.dur_secs as f64 + track_time.dur_frac) / 60.0).round() as u64,
                    track_time.dur_secs % 60
                );
                value
            } else if let Some(track_time) = &self.cached_track_time {
                current_time = format!(
                    "{:02}:{:02}",
                    ((track_time.ts_secs as f64 + track_time.ts_frac) / 60.0).round() as u64,
                    track_time.ts_secs % 60
                );
                total_time = format!(
                    "{:02}:{:02}",
                    ((track_time.dur_secs as f64 + track_time.dur_frac) / 60.0).round() as u64,
                    track_time.dur_secs % 60
                );
                (track_time.ts_secs as f64 + track_time.ts_frac)
                    / (track_time.dur_secs as f64 + track_time.dur_frac)
            } else {
                current_time = String::from("00:00");
                total_time = String::from("00:00");
                0.0
            };
            ui.horizontal(|ui| {
                ui.label(current_time);
                let slider = Slider::new(&mut self.time, 0.0..=1.0)
                    .orientation(SliderOrientation::Horizontal)
                    .show_value(false)
                    .ui(ui);
                ui.label(total_time);
                ui.add_space(10.0);
                let volume_slider = Slider::new(&mut self.volume, 0.0..=1.0)
                    .show_value(false)
                    .ui(ui);
                ui.label(format!("{}%", (self.volume * 100.0).round() as usize));
                self.slider_seek(slider, track_time.clone());
                if volume_slider.changed() {
                    self.player.set_volume(self.volume).unwrap();
                }
            });
            ui.horizontal(|ui| {
                ScrollArea::horizontal().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(from_path_to_name_without_ext(
                        self.player.current_track_name(),
                    ));
                    ui.add_space(10.0);
                    let text_toggle = if !self.player.is_playing() || self.player.is_paused() {
                        "▶"
                    } else {
                        "⏸"
                    };
                    let previous_btn = Button::new("⏮").frame(false).ui(ui);
                    let toggle_btn = Button::new(text_toggle).frame(false).ui(ui);
                    let next_btn = Button::new("⏭").frame(false).ui(ui);
                    if previous_btn.clicked() {
                        previous = true;
                    }
                    if toggle_btn.clicked() {
                        pause = true;
                    }
                    if next_btn.clicked() {
                        next = true;
                    }
                });
            });
        });
        if pause {
            self.toggle_pause(ctx);
        }
        if next {
            self.play_next(ctx);
        }
        if previous {
            self.play_previous(ctx);
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::warn_if_debug_build(ui);
            let row_height = 40.0;
            let total_rows = self.files.len();
            ScrollArea::vertical().show_rows(ui, row_height, total_rows, |ui, rows_range| {
                for i in rows_range {
                    let track = &self.files[i];
                    let name = &track.name;
                    let duration = &track.duration;
                    let mut update_title = false;
                    ui.horizontal(|ui| {
                        let mut frame = false;
                        if self.player.is_playing()
                            && &self.player.current_track_name().rsplit_once('.').unwrap().0 == name
                        {
                            ui.add_space(10.0);
                            frame = true;
                        }
                        ui.vertical(|ui| {
                            let button = Button::new(name).frame(frame).ui(ui);
                            if button.clicked() {
                                let index = self.player.get_index_from_track_name(name).unwrap();
                                self.player.end_current().unwrap();
                                self.player.play_index(index);
                                update_title = true;
                            }
                            ui.label(&track.artist);
                        });
                        ui.add(Label::new(format!(
                            "{:02}:{:02}",
                            duration / 60,
                            duration % 60
                        )));
                    });
                    if i + 1 != total_rows {
                        ui.separator();
                    }
                    if update_title {
                        self.update_title(ctx);
                    }
                }
                ui.allocate_space(ui.available_size());
            });
        });
        if !self.player.is_paused() {
            ctx.request_repaint_after(Duration::from_millis(750));
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        storage.set_string("track_files", toml::to_string(&self.files).unwrap());
        storage.set_string("volume", self.volume.to_string());
        if let Some(path) = &self.path {
            storage.set_string("path", path.to_string());
        }
    }

    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        self.player.end_current().unwrap();
    }
}

#[cfg(target_os = "linux")]
impl RootInterface for App {
    async fn raise(&self) -> mpris_server::zbus::fdo::Result<()> {
        Ok(())
    }

    async fn quit(&self) -> mpris_server::zbus::fdo::Result<()> {
        Ok(())
    }

    async fn can_quit(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn fullscreen(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn set_fullscreen(&self, fullscreen: bool) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn can_set_fullscreen(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn can_raise(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn has_track_list(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn identity(&self) -> mpris_server::zbus::fdo::Result<String> {
        Ok(String::from("N Music"))
    }

    async fn desktop_entry(&self) -> mpris_server::zbus::fdo::Result<String> {
        Ok(String::from("N Music.desktop"))
    }

    async fn supported_uri_schemes(&self) -> mpris_server::zbus::fdo::Result<Vec<String>> {
        Ok(vec![])
    }

    async fn supported_mime_types(&self) -> mpris_server::zbus::fdo::Result<Vec<String>> {
        Ok(vec![])
    }
}

#[cfg(target_os = "linux")]
impl PlayerInterface for App {
    async fn next(&self) -> mpris_server::zbus::fdo::Result<()> {
        todo!()
    }

    async fn previous(&self) -> mpris_server::zbus::fdo::Result<()> {
        todo!()
    }

    async fn pause(&self) -> mpris_server::zbus::fdo::Result<()> {
        todo!()
    }

    async fn play_pause(&self) -> mpris_server::zbus::fdo::Result<()> {
        todo!()
    }

    async fn stop(&self) -> mpris_server::zbus::fdo::Result<()> {
        todo!()
    }

    async fn play(&self) -> mpris_server::zbus::fdo::Result<()> {
        todo!()
    }

    async fn seek(&self, offset: Time) -> mpris_server::zbus::fdo::Result<()> {
        todo!()
    }

    async fn set_position(
        &self,
        track_id: TrackId,
        position: Time,
    ) -> mpris_server::zbus::fdo::Result<()> {
        todo!()
    }

    async fn open_uri(&self, uri: String) -> mpris_server::zbus::fdo::Result<()> {
        todo!()
    }

    async fn playback_status(&self) -> mpris_server::zbus::fdo::Result<PlaybackStatus> {
        todo!()
    }

    async fn loop_status(&self) -> mpris_server::zbus::fdo::Result<LoopStatus> {
        todo!()
    }

    async fn set_loop_status(&self, loop_status: LoopStatus) -> mpris_server::zbus::Result<()> {
        todo!()
    }

    async fn rate(&self) -> mpris_server::zbus::fdo::Result<PlaybackRate> {
        todo!()
    }

    async fn set_rate(&self, rate: PlaybackRate) -> mpris_server::zbus::Result<()> {
        todo!()
    }

    async fn shuffle(&self) -> mpris_server::zbus::fdo::Result<bool> {
        todo!()
    }

    async fn set_shuffle(&self, shuffle: bool) -> mpris_server::zbus::Result<()> {
        todo!()
    }

    async fn metadata(&self) -> mpris_server::zbus::fdo::Result<Metadata> {
        todo!()
    }

    async fn volume(&self) -> mpris_server::zbus::fdo::Result<Volume> {
        todo!()
    }

    async fn set_volume(&self, volume: Volume) -> mpris_server::zbus::Result<()> {
        todo!()
    }

    async fn position(&self) -> mpris_server::zbus::fdo::Result<Time> {
        todo!()
    }

    async fn minimum_rate(&self) -> mpris_server::zbus::fdo::Result<PlaybackRate> {
        todo!()
    }

    async fn maximum_rate(&self) -> mpris_server::zbus::fdo::Result<PlaybackRate> {
        todo!()
    }

    async fn can_go_next(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn can_go_previous(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn can_play(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn can_pause(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn can_seek(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn can_control(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }
}
