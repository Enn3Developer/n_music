#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[tokio::main]
async fn main() {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    use n_player::platform::DesktopPlatform;
    #[cfg(target_os = "linux")]
    use n_player::platform::LinuxPlatform;
    use n_player::settings::Settings;

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let platform = DesktopPlatform {};
    #[cfg(target_os = "linux")]
    let platform = LinuxPlatform::new();
    let settings = Settings::read_saved(&platform);
    n_player::app::run_app(settings, platform).await
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn main() {}
