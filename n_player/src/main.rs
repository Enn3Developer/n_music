#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use eframe::egui;
use eframe::egui::FontFamily;
use mpris_server::Server;
use n_audio::queue::QueuePlayer;
use n_player::bus_server::linux::MPRISBridge;
use n_player::runner::{run, Runner};
use n_player::ui::app::App;
use n_player::{add_all_tracks_to_player, bus_server, find_cjk_font};
use std::fs;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::sync::RwLock;

fn main() {
    let tmp = NamedTempFile::new().unwrap();
    let (tx, rx) = flume::unbounded();

    let mut player = QueuePlayer::new(String::from("/home/enn3/Music"));
    add_all_tracks_to_player(&mut player, String::from("/home/enn3/Music"));

    let runner = Arc::new(RwLock::new(Runner::new(rx, player)));

    let r = runner.clone();
    let tx_t = tx.clone();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let future = runtime.spawn(async move {
        #[cfg(target_os = "linux")]
        let server = Some(
            Server::new("n_music", MPRISBridge::new(r.clone(), tx_t.clone()))
                .await
                .unwrap(),
        );
        #[cfg(not(target_os = "linux"))]
        let server = None;

        let runner_future = tokio::task::spawn(run(r.clone()));
        let bus_future = tokio::task::spawn(bus_server::run(server, r.clone(), tmp));

        let _ = tokio::join!(runner_future, bus_future);
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "N Music",
        native_options,
        Box::new(|cc| {
            let font_file = find_cjk_font().unwrap();
            let font_name = font_file
                .split('/')
                .last()
                .unwrap()
                .split('.')
                .next()
                .unwrap()
                .to_string();
            let font_file_bytes = fs::read(font_file).unwrap();

            let font_data = egui::FontData::from_owned(font_file_bytes);
            let mut font_def = egui::FontDefinitions::default();
            font_def.font_data.insert(font_name.to_string(), font_data);

            font_def
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .push(font_name);

            cc.egui_ctx.set_fonts(font_def);
            Ok(Box::new(App::new(runner.clone(), tx, cc)))
        }),
    )
    .expect("can't start app");

    future.abort();

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
