pub mod modules;

use crate::app::modules::{
    loader::LoaderModule, updater::UpdaterModule, window::WindowModule, Module, ModuleMessage,
};
use crate::runner::{run, RunnerMessage};
use crate::{bus_server, WindowSize};
use flume::{Receiver, Sender};
use n_audio::queue::QueuePlayer;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub enum AppMessage<P: crate::platform::Platform + Send + 'static> {
    RunnerMessage(RunnerMessage),
    UpdaterMessage(<UpdaterModule<P> as Module<P>>::Message),
    WindowMessage(<WindowModule<P> as Module<P>>::Message),
    LoaderMessage(<LoaderModule<P> as Module<P>>::Message),
    RequestQuit,
}

// Useless warning, it may be fixed in a future Rust edition
#[allow(type_alias_bounds)]
pub type Platform<P: crate::platform::Platform + Send> = Arc<Mutex<P>>;
pub type Settings = Arc<Mutex<crate::settings::Settings>>;
pub type Runner = Arc<RwLock<crate::runner::Runner>>;
#[allow(type_alias_bounds)]
pub type Messenger<P: crate::platform::Platform + Send + 'static> = Sender<AppMessage<P>>;

pub async fn run_app<P: crate::platform::Platform + Send + 'static>(
    settings: crate::settings::Settings,
    platform: P,
) {
    let platform: Platform<P> = Arc::new(Mutex::new(platform));
    let settings: Settings = Arc::new(Mutex::new(settings));

    let (tx_runner, rx) = flume::unbounded();

    let player = QueuePlayer::new(settings.lock().await.path.clone());
    let runner = Arc::new(RwLock::new(crate::runner::Runner::new(player)));

    let r = runner.clone();
    let tx_t = tx_runner.clone();

    let p = platform.clone();
    p.lock().await.add_runner(r.clone(), tx_t.clone()).await;
    let future = tokio::spawn(async move {
        let runner_future = tokio::task::spawn(run(r.clone(), rx));
        let bus_future = tokio::task::spawn(bus_server::run(p, r.clone()));
        let _ = tokio::join!(runner_future, bus_future);
    });

    let (messenger, receiver) = flume::unbounded();

    let (tx_loader, _rx_loader) = flume::unbounded();
    let p = platform.clone();
    let s = settings.clone();
    let r = runner.clone();
    let m = messenger.clone();
    let loader = tokio::task::spawn(async move {
        let loader = LoaderModule::setup(p, s, r, m, _rx_loader).await;
        loader.start().await;
    });

    let (tx_updater, _rx_updater) = flume::unbounded();
    let p = platform.clone();
    let s = settings.clone();
    let r = runner.clone();
    let m = messenger.clone();
    let updater = tokio::task::spawn(async move {
        let updater = UpdaterModule::setup(p, s, r, m, _rx_updater).await;
        updater.start().await;
    });

    let (tx_window, _rx_window) = flume::unbounded();
    let p = platform.clone();
    let s = settings.clone();
    let r = runner.clone();
    let m = messenger.clone();
    let window = WindowModule::setup(p, s, r, m, _rx_window).await;

    let message_parser = tokio::task::spawn(async move {
        message_parser(tx_loader, tx_updater, tx_window, tx_runner, receiver).await
    });

    tokio::task::block_in_place(|| window.start_sync());

    message_parser.await.unwrap();
    loader.await.unwrap();
    updater.await.unwrap();

    settings.lock().await.volume = runner.read().await.volume();
    if settings.lock().await.save_window_size {
        // TODO: add save window size back
        // let width = main_window.get_last_width() as usize;
        // let height = main_window.get_last_height() as usize;
        // settings.lock().await.window_size = WindowSize { width, height };
    } else {
        settings.lock().await.window_size = WindowSize::default();
    }

    future.abort();
    settings.lock().await.save(platform.lock().await).await;
}

async fn message_parser<P: crate::platform::Platform + Send + 'static>(
    tx_loader: Sender<<LoaderModule<P> as Module<P>>::Message>,
    tx_updater: Sender<<UpdaterModule<P> as Module<P>>::Message>,
    tx_window: Sender<<WindowModule<P> as Module<P>>::Message>,
    tx_runner: Sender<RunnerMessage>,
    receiver: Receiver<AppMessage<P>>,
) {
    'exit: loop {
        while let Ok(message) = receiver.recv_async().await {
            match message {
                AppMessage::RunnerMessage(m) => tx_runner.send_async(m).await.unwrap(),
                AppMessage::UpdaterMessage(m) => tx_updater.send_async(m).await.unwrap(),
                AppMessage::WindowMessage(m) => tx_window.send_async(m).await.unwrap(),
                AppMessage::LoaderMessage(m) => tx_loader.send_async(m).await.unwrap(),
                AppMessage::RequestQuit => {
                    tx_loader
                        .send_async(<LoaderModule<P> as Module<P>>::Message::quit())
                        .await
                        .unwrap();
                    tx_updater
                        .send_async(<UpdaterModule<P> as Module<P>>::Message::quit())
                        .await
                        .unwrap();
                    tx_window
                        .send_async(<WindowModule<P> as Module<P>>::Message::quit())
                        .await
                        .unwrap();
                    break 'exit;
                }
            }
        }
    }
}
