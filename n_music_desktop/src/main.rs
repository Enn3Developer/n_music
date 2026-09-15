#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

slint::include_modules!();

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod app;
mod localization;
mod platform;
mod scenes;
mod ui;

fn main() {
    // Installer hooks must finish before starting Tokio, audio, or the UI.
    velopack::VelopackApp::build().run();
    run();
}

#[tokio::main]
async fn run() {
    use n_music_core::settings::Settings;

    let platform = platform::DesktopPlatform::new();

    let settings = Settings::read_saved(&platform).await;
    let (writer, rx) = n_event_bus::EventWriter::channel();
    app::run(settings, platform, writer, rx, Vec::new()).await
}
