#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use eframe::egui;
#[cfg(target_os = "linux")]
use mpris_server::Server;
use n_player::app::App;
#[cfg(target_os = "linux")]
use n_player::mpris_server::MPRISServer;
#[cfg(target_os = "linux")]
use pollster::FutureExt;

fn main() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 600.0]),
        ..Default::default()
    };

    let (tx, rx) = flume::unbounded();
    let (tx_c, rx_c) = flume::unbounded();

    #[cfg(target_os = "linux")]
    let server = Server::new("com.enn3developer.n_music", MPRISServer::new(tx, rx_c))
        .block_on()
        .unwrap();

    eframe::run_native(
        "N Music",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(App::new(
                cc,
                rx,
                tx_c,
                #[cfg(target_os = "linux")]
                server,
            )))
        }),
    )
    .expect("Can't start app");
}
