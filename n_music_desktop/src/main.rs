#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

#[cfg(all(not(target_os = "android"), not(target_env = "msvc")))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod platform;
#[cfg(target_os = "linux")]
mod mpris;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() {
    // Installer hooks must finish before starting Tokio, audio, or the UI.
    velopack::VelopackApp::build().run();
    run();
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[tokio::main]
async fn run() {
    use n_player::settings::Settings;

    #[cfg(target_os = "linux")]
    let platform = platform::LinuxPlatform::new();
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let platform = platform::DesktopPlatform {};

    let settings = Settings::read_saved(&platform).await;
    let (writer, rx) = n_event_bus::EventWriter::channel();

    #[cfg(target_os = "linux")]
    let bridge = mpris::MprisBridge::new(writer.clone(), settings.volume).await;

    let setup = move |app: &mut n_event_bus::App| {
        let _ = app;
        #[cfg(target_os = "linux")]
        if let Some(bridge) = bridge {
            app.register_subscriber(bridge);
        }
    };

    n_player::app::run_app_with_events(settings, platform, writer, rx, Vec::new(), setup).await
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn main() {}
