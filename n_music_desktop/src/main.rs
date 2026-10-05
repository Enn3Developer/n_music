#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

mod bridge;
mod bus;
mod format;
mod hub;
mod i18n;
mod listener;
mod platform;
mod query;
mod settings;
mod worker;

use cxx_qt::casting::Upcast;
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};
use n_music_core::logging;
use n_music_core::settings::JsonFileStorage;
use std::sync::Arc;

fn main() {
    let data_dir = platform::internal_dir();
    let _logging = match logging::init(&data_dir) {
        Ok(logging) => Some(logging),
        Err(error) => {
            eprintln!("Could not initialize logging: {error}");
            None
        }
    };

    // Install reporting first, but finish installer hooks before starting audio or the UI.
    velopack::VelopackApp::build().run();

    let storage = Arc::new(JsonFileStorage::open(data_dir.join("settings.json")));
    settings::load(storage.clone());
    let bus = bus::Bus::start(storage.clone(), &data_dir, &platform::cache_dir());

    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        app.as_mut().set_application_name(&QString::from("N Music"));
        app.set_application_version(&QString::from(env!("CARGO_PKG_VERSION")));
    }
    QGuiApplication::set_desktop_file_name(&QString::from("n_music"));
    bridge::icon::install();
    let mut engine = QQmlApplicationEngine::new();
    if let Some(mut engine) = engine.as_mut() {
        bridge::covers::install(engine.as_mut().upcast_pin());
        engine.load(&QUrl::from("qrc:/qt/qml/NMusic/qml/Main.qml"));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }

    bus.stop();
    drop(engine);
    storage.flush();
    log::info!("Application stopped");
}
