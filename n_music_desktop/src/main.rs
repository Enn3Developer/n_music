#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use n_music_core::logging;
use n_music_core::platform::Platform;
use n_music_core::settings::Settings;

slint::include_modules!();

mod app;
mod localization;
mod platform;
mod scenes;
mod ui;

fn main() {
    run();
}

fn run() {
    let platform = platform::DesktopPlatform::new();
    let _logging = match logging::init(&platform.internal_dir()) {
        Ok(logging) => Some(logging),
        Err(error) => {
            eprintln!("Could not initialize logging: {error}");
            None
        }
    };

    // Install reporting first, but finish installer hooks before starting audio or the UI.
    velopack::VelopackApp::build().run();

    let settings = Settings::read_saved(&platform);
    let (writer, rx) = n_event_bus::EventWriter::channel();
    app::run(settings, platform, writer, rx, Vec::new());
    log::info!("Application stopped");
}
