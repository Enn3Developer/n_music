#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use n_player::app::run_app;

#[tokio::main]
async fn main() {
    run_app().await
}
