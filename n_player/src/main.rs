#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use pollster::FutureExt;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    use n_player::platform::DesktopPlatform;
    #[cfg(target_os = "linux")]
    use n_player::platform::LinuxPlatform;
    use n_player::settings::Settings;

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let platform = DesktopPlatform {};
    #[cfg(target_os = "linux")]
    let platform = LinuxPlatform::new();
    // let settings = Settings::read_saved(&platform).await;
    let settings = Settings::default();
    n_player::app::run_app(settings, platform)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn main() {}
