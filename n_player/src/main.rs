#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use n_player::settings::Settings;

#[tokio::main]
async fn main() {
    let settings = Settings::read_saved().await;
    n_player::app::run_app(settings).await
}
