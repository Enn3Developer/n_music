#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

#[tokio::main]
async fn main() {
    n_player::app::run_app().await
}
