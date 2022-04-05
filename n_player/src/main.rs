#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use eframe::emath::Vec2;

use n_audio::Player;
use n_player::{add_all_tracks_to_player, app, Config};

const PATH: &str = "./.nmusic.toml";

// fn open_window_asking_path() -> String {
//     let mut path = String::new();
//
//     let mut wind = Window::new(100, 100, 400, 150, "Choose music dir");
//
//     Frame::new(
//         125,
//         10,
//         150,
//         50,
//         "Please choose a directory where the music is stored",
//     );
//     let mut button = Button::new(135, 60, 130, 50, "Select directory");
//     let mut dialog = FileDialog::new(FileDialogType::BrowseDir);
//
//     wind.end();
//     wind.show();
//
//     button.set_callback(move |_| {
//         dialog.show();
//         tx.send(dialog.filename().to_str().unwrap().to_string());
//     });
//
//     while app.wait() {
//         if let Some(message) = rx.recv() {
//             path = message;
//             app::quit();
//         }
//     }
//
//     path
// }

fn main() {
    let mut config = if Config::exists(PATH) {
        Config::load(PATH).expect("Can't load config file")
    } else {
        Config::new()
    };
    // if config.path().is_none() {
    //     let path = open_window_asking_path();
    //     println!("{}", path);
    //     config.set_path(path);
    // }
    config.save(PATH).expect("Can't save config file");

    let mut player = Player::new("N Music".to_string());

    add_all_tracks_to_player(&mut player, &config);

    let app = app::App::new(config, PATH.to_string(), player);

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(Vec2::new(400.0, 600.0)),
        ..Default::default()
    };

    eframe::run_native(Box::new(app), native_options);
}
