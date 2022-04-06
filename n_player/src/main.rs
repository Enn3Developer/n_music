#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use eframe::emath::Vec2;
use native_dialog::FileDialog;

use n_audio::player::Player;
use n_player::app::App;
use n_player::{add_all_tracks_to_player, Config};

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

    let mut player = Player::new("N Music".to_string());

    add_all_tracks_to_player(&mut player, &config);

    let app = App::new(config, PATH.to_string(), player);

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(Vec2::new(400.0, 600.0)),
        ..Default::default()
    };

    eframe::run_native(Box::new(app), native_options);
}
