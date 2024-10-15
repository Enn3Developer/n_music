#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[tokio::main]
async fn main() {
    use n_player::platform::DesktopPlatform;
    use n_player::settings::Settings;

    let platform = DesktopPlatform {};
    let settings = Settings::read_saved(&platform);
    n_player::app::run_app(settings, platform).await
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn main() {}
