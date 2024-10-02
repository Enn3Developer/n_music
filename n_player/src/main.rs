#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use eframe::egui;
use mpris_server::Server;
use n_audio::queue::QueuePlayer;
use n_player::bus_server::linux::MPRISBridge;
use n_player::runner::{run, Runner};
use n_player::{add_all_tracks_to_player, bus_server};
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 600.0]),
        ..Default::default()
    };

    let tmp = NamedTempFile::new().unwrap();
    let (tx, rx) = flume::unbounded();

    let mut player = QueuePlayer::new(String::from("/home/enn3/Music"));
    add_all_tracks_to_player(&mut player, String::from("/home/enn3/Music"));

    let runner = Arc::new(RwLock::new(Runner::new(rx, player)));
    #[cfg(target_os = "linux")]
    let server = Some(
        Server::new("n_music", MPRISBridge::new(runner.clone(), tx))
            .await
            .unwrap(),
    );
    #[cfg(not(target_os = "linux"))]
    let server = None;

    tokio::join!(run(runner.clone()), bus_server::run(server, runner, tmp));

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
