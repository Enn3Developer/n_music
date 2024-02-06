#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use eframe::{egui, HardwareAcceleration};
use n_audio::queue::QueuePlayer;
use n_player::app::App;
use n_player::{add_all_tracks_to_player, Config};
use native_dialog::FileDialog;

const PATH: &str = "./.nmusic.toml";

fn main() {
    let mut config = if Config::exists(PATH) {
        Config::load(PATH).expect("Can't load config file")
    } else {
        Config::new()
    };

    if config.path().is_none() {
        let dir = FileDialog::default()
            .show_open_single_dir()
            .unwrap()
            .unwrap();
        config.set_path(dir.to_str().unwrap().to_string());

        config.save(PATH).expect("Can't save config file");
    }

    let mut player: QueuePlayer<String> = QueuePlayer::new();

    add_all_tracks_to_player(&mut player, &config);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 600.0]),
        hardware_acceleration: HardwareAcceleration::Preferred,
        ..Default::default()
    };

    eframe::run_native(
        "N Music",
        native_options,
        Box::new(|cc| Box::new(App::new(config, PATH.to_string(), player, cc))),
    )
    .expect("Can't start app");
}
