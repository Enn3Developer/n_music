#[cfg(target_os = "linux")]
use crate::mpris_server::MPRISServer;
use crate::{
    add_all_tracks_to_player, get_image, loader_thread, vec_contains, ClientMessage, FileTrack,
    FileTracks, Message, ServerMessage,
};
use eframe::egui::{
    Button, Event, Image, Key, Label, Modifiers, Response, ScrollArea, Slider, SliderOrientation,
    ViewportCommand, Visuals, Widget,
};
use eframe::epaint::FontFamily;
use eframe::{egui, Storage};
use flume::{Receiver, Sender};
use hashbrown::HashMap;
use image::imageops::FilterType;
use image::ImageFormat;
#[cfg(target_os = "linux")]
use mpris_server::zbus::zvariant::ObjectPath;
#[cfg(target_os = "linux")]
use mpris_server::Server;
#[cfg(target_os = "linux")]
use mpris_server::{PlaybackStatus, Property};
use n_audio::queue::QueuePlayer;
use n_audio::{remove_ext, TrackTime};
use pollster::FutureExt;
use std::fs::DirEntry;
use std::io::{Cursor, Seek, Write};
#[cfg(target_os = "windows")]
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::{fs, thread};
use tempfile::NamedTempFile;

pub struct App {
    path: Option<String>,
    player: QueuePlayer,
    volume: f32,
    time: f64,
    check_metadata: bool,
    cached_track_time: Option<TrackTime>,
    files: FileTracks,
    rx: Option<Receiver<Message>>,
    rx_server: Receiver<ServerMessage>,
    tx_server: Sender<ClientMessage>,
    loaded_images: HashMap<usize, Vec<u8>>,
    tmp: NamedTempFile,
    #[cfg(target_os = "linux")]
    server: Server<MPRISServer>,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        rx_server: Receiver<ServerMessage>,
        tx_server: Sender<ClientMessage>,
        tmp: NamedTempFile,
        #[cfg(target_os = "linux")] server: Server<MPRISServer>,
    ) -> Self {
        Self::configure_fonts(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
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
            check_metadata: false,
            cached_track_time: None,
            files,
            rx,
            rx_server,
            tx_server,
            loaded_images: HashMap::new(),
            tmp,
            #[cfg(target_os = "linux")]
            server,
        }
    }

    fn slider_seek(&mut self, slider: Response, track_time: Option<TrackTime>) {
        if let Some(track_time) = track_time {
            if slider.changed() {
                self.player.pause().unwrap();
                let total_time = track_time.len_secs as f64 + track_time.len_frac;
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
                let mut name = remove_ext(&entry.path());
                name.shrink_to_fit();
                let contains = vec_contains(saved_files, &name);
                let (duration, mut artist) = if contains.0 {
                    (
                        saved_files[contains.1].length,
                        saved_files[contains.1].artist.clone(),
                    )
                } else {
                    (0, "ARTIST".to_string())
                };
                artist.shrink_to_fit();
                files.push(FileTrack {
                    name,
                    length: duration,
                    artist,
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
            #[cfg(target_os = "linux")]
            self.server
                .properties_changed([Property::PlaybackStatus(PlaybackStatus::Playing)])
                .block_on()
                .unwrap();
        } else {
            self.player.pause().unwrap();
            #[cfg(target_os = "linux")]
            self.server
                .properties_changed([Property::PlaybackStatus(PlaybackStatus::Paused)])
                .block_on()
                .unwrap();
        }
        if !self.player.is_playing() {
            self.play_next(ctx);
        }
    }

    fn play_next(&mut self, ctx: &egui::Context) {
        self.player.end_current().unwrap();
        self.player.play_next();
        self.update_title(ctx);
        self.check_metadata = true;
    }

    fn play_previous(&mut self, ctx: &egui::Context) {
        if let Some(cached_track_time) = &self.cached_track_time {
            if cached_track_time.pos_secs < 2 {
                self.player.seek_to(0, 0.0).unwrap();
            } else {
                self.player.end_current().unwrap();
                self.player.play_previous();
                self.update_title(ctx);
                self.check_metadata = true;
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
                        self.files[i].length = dur;
                    }
                    Message::Artist(i, artist) => {
                        self.files[i].artist = artist;
                    }
                }
            }
        }
        if self.player.has_ended() {
            self.player.play_next();
            self.update_title(ctx);
            self.check_metadata = true;
        }

        let mut pause = false;
        let mut next = false;
        let mut previous = false;
        let mut send_time = false;
        let mut send_server = false;

        while let Ok(message) = self.rx_server.try_recv() {
            match message {
                ServerMessage::PlayNext => next = true,
                ServerMessage::PlayPrevious => previous = true,
                ServerMessage::TogglePause => pause = true,
                ServerMessage::SetVolume(volume) => {
                    self.volume = volume as f32;
                    self.player.set_volume(self.volume).unwrap();
                }
                ServerMessage::AskVolume => self
                    .tx_server
                    .send(ClientMessage::Volume(self.volume as f64))
                    .unwrap(),
                ServerMessage::AskPlayback => self
                    .tx_server
                    .send(ClientMessage::Playback(self.player.is_playing()))
                    .unwrap(),
                ServerMessage::AskMetadata => {
                    self.check_metadata = true;
                    send_server = true;
                }
                ServerMessage::AskTime => send_time = true,
                ServerMessage::Pause => {
                    if self.player.is_playing() {
                        pause = true;
                    }
                }
                ServerMessage::Play => {
                    if self.player.is_paused() || !self.player.is_playing() {
                        pause = true;
                    }
                }
            }
        }

        if self.check_metadata {
            self.check_metadata = false;
            let mut track = None;
            for file_track in &self.files.tracks {
                if remove_ext(self.player.current_track_name()) == file_track.name {
                    track = Some(file_track.clone());
                }
            }
            let image = get_image(self.player.get_path_for_file(
                if self.player.index() == usize::MAX - 1 {
                    0
                } else {
                    self.player.index()
                },
            ));
            if !image.is_empty() {
                self.tmp.rewind().unwrap();
                self.tmp.write_all(&image).unwrap();
            }
            let (title, artist, time, path, image_path) = match track {
                None => (None, None, 0, String::from("/empty"), None),
                Some(track) => (
                    Some(track.name.clone()),
                    Some(vec![track.artist]),
                    track.length,
                    "/n_music".to_string(),
                    if image.is_empty() {
                        None
                    } else {
                        Some(self.tmp.path().to_str().unwrap().to_string())
                    },
                ),
            };

            if send_server {
                self.tx_server
                    .send(ClientMessage::Metadata(
                        title.clone(),
                        artist.clone(),
                        time,
                        path.clone(),
                        image_path.clone(),
                    ))
                    .unwrap();
            }
            #[cfg(target_os = "linux")]
            {
                let mut meta = mpris_server::Metadata::new();
                meta.set_artist(artist);
                meta.set_title(title);
                meta.set_length(Some(mpris_server::Time::from_secs(time as i64)));
                meta.set_trackid(Some(ObjectPath::from_string_unchecked(
                    path.replace(" ", "_"),
                )));
                meta.set_art_url(image_path);
                self.server
                    .properties_changed([
                        Property::Metadata(meta),
                        Property::PlaybackStatus(PlaybackStatus::Playing),
                    ])
                    .block_on()
                    .unwrap();
            }
        }

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
                let value = (track_time.pos_secs as f64 + track_time.pos_frac)
                    / (track_time.len_secs as f64 + track_time.len_frac);
                self.cached_track_time = Some(track_time.clone());
                if send_time {
                    self.tx_server
                        .send(ClientMessage::Time(track_time.pos_secs))
                        .unwrap();
                }
                current_time = format!(
                    "{:02}:{:02}",
                    ((track_time.pos_secs as f64 + track_time.pos_frac) / 60.0).floor() as u64,
                    track_time.pos_secs % 60
                );
                total_time = format!(
                    "{:02}:{:02}",
                    ((track_time.len_secs as f64 + track_time.len_frac) / 60.0).floor() as u64,
                    track_time.len_secs % 60
                );
                value
            } else if let Some(track_time) = &self.cached_track_time {
                if send_time {
                    self.tx_server
                        .send(ClientMessage::Time(track_time.pos_secs))
                        .unwrap();
                }
                current_time = format!(
                    "{:02}:{:02}",
                    ((track_time.pos_secs as f64 + track_time.pos_frac) / 60.0).floor() as u64,
                    track_time.pos_secs % 60
                );
                total_time = format!(
                    "{:02}:{:02}",
                    ((track_time.len_secs as f64 + track_time.len_frac) / 60.0).floor() as u64,
                    track_time.len_secs % 60
                );
                (track_time.pos_secs as f64 + track_time.pos_frac)
                    / (track_time.len_secs as f64 + track_time.len_frac)
            } else {
                current_time = String::from("00:00");
                total_time = String::from("00:00");
                if send_time {
                    self.tx_server.send(ClientMessage::Time(0)).unwrap();
                }
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
                    self.server
                        .properties_changed([Property::Volume(self.volume as f64)])
                        .block_on()
                        .unwrap();
                }
            });
            ui.horizontal(|ui| {
                ScrollArea::horizontal().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(remove_ext(self.player.current_track_name()));
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
                let mut keys = vec![];
                for loaded in self.loaded_images.keys() {
                    if !rows_range.contains(loaded) {
                        keys.push(loaded.clone());
                    }
                }
                for key in keys {
                    self.loaded_images.remove(&key);
                }
                for i in rows_range {
                    let track = &self.files[i];
                    let name = &track.name;
                    let length = &track.length;
                    let mut update_title = false;

                    if !self.loaded_images.contains_key(&i) {
                        let mut image = get_image(self.player.get_path_for_file(
                            self.player.get_index_from_track_name(name).unwrap(),
                        ));
                        if !image.is_empty() {
                            image::load_from_memory(&image)
                                .unwrap()
                                .resize(32, 32, FilterType::Lanczos3)
                                .write_to(&mut Cursor::new(&mut image), ImageFormat::Png)
                                .unwrap();
                        }
                        self.loaded_images.insert(i, image.clone());
                    }

                    ui.horizontal(|ui| {
                        let cover = self.loaded_images.get(&i).unwrap();
                        if !cover.is_empty() {
                            Image::from_bytes(
                                format!("bytes://{}", name.escape_default()),
                                cover.clone(),
                            )
                            .fit_to_original_size(1.0)
                            .ui(ui);
                        }
                        let mut frame = false;
                        if self.player.is_playing()
                            && &remove_ext(self.player.current_track_name()) == name
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
                                self.check_metadata = true;
                            }
                            ui.label(&track.artist);
                        });
                        ui.add(Label::new(format!("{:02}:{:02}", length / 60, length % 60)));
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

        #[cfg(target_os = "linux")]
        if !self.player.is_playing() {
            self.server
                .properties_changed([Property::PlaybackStatus(PlaybackStatus::Paused)])
                .block_on()
                .unwrap();
        }

        ctx.request_repaint_after(Duration::from_millis(300));
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
