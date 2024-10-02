#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use eframe::egui;
use tempfile::NamedTempFile;

#[tokio::main]
async fn main() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 600.0]),
        ..Default::default()
    };

    let tmp = NamedTempFile::new().unwrap();

    // eframe::run_native(
    //     "N Music",
    //     native_options,
    //     Box::new(|cc| {
    //         Ok(Box::new(App::new(
    //             cc,
    //             rx,
    //             tx_c,
    //             tmp,
    //             #[cfg(target_os = "linux")]
    //             server,
    //         )))
    //     }),
    // )
    // .expect("Can't start app");
}
