#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

slint::include_modules!();

mod app;
mod localization;
mod platform;
mod scenes;
mod ui;

fn main() {
    // Installer hooks must finish before starting audio or the UI.
    velopack::VelopackApp::build().run();
    run();
}

fn run() {
    use n_music_core::settings::Settings;

    let platform = platform::DesktopPlatform::new();

    let settings = Settings::read_saved(&platform);
    let (writer, rx) = n_event_bus::EventWriter::channel();
    app::run(settings, platform, writer, rx, Vec::new())
}
