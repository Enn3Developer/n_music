#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use flume::{Receiver, Sender};
use image::imageops::FilterType;
use image::ImageFormat;
#[cfg(target_os = "linux")]
use mpris_server::Server;
use n_audio::music_track::MusicTrack;
use n_audio::queue::QueuePlayer;
use n_audio::remove_ext;
#[cfg(target_os = "linux")]
use n_player::bus_server::linux::MPRISBridge;
#[cfg(not(target_os = "linux"))]
use n_player::bus_server::DummyServer;
use n_player::runner::{run, Runner, RunnerMessage, RunnerSeek};
use n_player::storage::Storage;
use n_player::{add_all_tracks_to_player, bus_server, get_image};
use slint::private_unstable_api::re_exports::ColorScheme;
use slint::VecModel;
use std::io::Cursor;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::sync::RwLock;

slint::include_modules!();

unsafe impl Send for TrackData {}
unsafe impl Sync for TrackData {}

async fn loader_task(
    runner: Arc<RwLock<Runner>>,
    tx: Sender<TrackData>,
    rx_l: Arc<tokio::sync::Mutex<Receiver<usize>>>,
) {
    loop {
        if let Ok(index) = rx_l.lock().await.recv_async().await {
            if index == usize::MAX {
                return;
            }
            let path = runner.read().await.get_path_for_file(index).await;
            if let Ok(track) = MusicTrack::new(path.to_string_lossy().to_string()) {
                if let Ok(meta) = track.get_meta() {
                    let p = path.clone();
                    let image_path = if let Ok(mut image) =
                        tokio::task::spawn_blocking(move || get_image(p)).await
                    {
                        if !image.is_empty() {
                            image::load_from_memory(&image)
                                .unwrap()
                                .resize_to_fill(128, 128, FilterType::Lanczos3)
                                .to_rgb8()
                                .write_to(&mut Cursor::new(&mut image), ImageFormat::Jpeg)
                                .unwrap();

                            let images_dir = Storage::app_dir().join("images");
                            if !images_dir.exists() {
                                tokio::fs::create_dir(images_dir.as_path()).await.unwrap();
                            }
                            let path = images_dir.join(format!("{}.jpg", remove_ext(path)));
                            tokio::fs::write(path.as_path(), image).await.unwrap();
                            path
                        } else {
                            PathBuf::new().join("thisdoesntexistsodontworryaboutit")
                        }
                    } else {
                        PathBuf::new().join("thisdoesntexistsodontworryaboutit")
                    };

                    tx.send_async(TrackData {
                        artist: meta.artist.into(),
                        time: format!(
                            "{:02}:{:02}",
                            (meta.time.length / 60.0).floor() as u64,
                            meta.time.length.floor() as u64 % 60
                        )
                        .into(),
                        cover: if image_path.exists() {
                            slint::Image::load_from_path(&image_path).unwrap()
                        } else {
                            Default::default()
                        },
                        title: meta.title.into(),
                        index: index as i32,
                    })
                    .await
                    .unwrap();
                }
            }
        }
    }
}

async fn loader(runner: Arc<RwLock<Runner>>, tx: Sender<TrackData>) {
    let len = runner.read().await.len();
    let mut tasks = vec![];
    let (tx_l, rx_l) = flume::unbounded();
    let rx_l = Arc::new(tokio::sync::Mutex::new(rx_l));
    for _ in 0..num_cpus::get() {
        let runner = runner.clone();
        let tx = tx.clone();
        let rx_l = rx_l.clone();
        tasks.push(tokio::task::spawn(loader_task(runner, tx, rx_l)));
    }
    for i in 0..len {
        tx_l.send_async(i).await.unwrap();
    }
    for _ in 0..num_cpus::get() {
        tx_l.send_async(usize::MAX).await.unwrap();
    }
    for task in tasks {
        task.await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    let storage = Rc::new(Mutex::new(Storage::read_saved()));

    let tmp = NamedTempFile::new().unwrap();
    let (tx, rx) = flume::unbounded();

    let mut player = QueuePlayer::new(storage.lock().unwrap().path.clone());
    add_all_tracks_to_player(&mut player, storage.lock().unwrap().path.clone()).await;
    let len = player.len();

    let runner = Arc::new(RwLock::new(Runner::new(player)));

    let r = runner.clone();
    let tx_t = tx.clone();

    let (tx_l, rx_l) = flume::unbounded();
    let main_window = MainWindow::new().unwrap();

    let future = tokio::spawn(async move {
        #[cfg(target_os = "linux")]
        let server = Server::new("n_music", MPRISBridge::new(r.clone(), tx_t.clone()))
            .await
            .unwrap();
        #[cfg(not(target_os = "linux"))]
        let server = DummyServer;

        let runner_future = tokio::task::spawn(run(r.clone(), rx));
        let bus_future = tokio::task::spawn(bus_server::run(server, r.clone(), tmp));
        let loader_future = tokio::task::spawn(loader(r.clone(), tx_l));

        let _ = tokio::join!(runner_future, bus_future, loader_future);
    });

    let mut tracks = vec![];
    for i in 0..len {
        let track_path = runner.read().await.get_path_for_file(i).await;
        tracks.push(TrackData {
            artist: Default::default(),
            cover: Default::default(),
            time: Default::default(),
            title: remove_ext(track_path).into(),
            index: i as i32,
        });
    }

    main_window.set_version(env!("CARGO_PKG_VERSION").into());
    main_window.set_color_scheme(ColorScheme::Dark);

    tokio::task::block_in_place(|| main_window.set_tracks(VecModel::from_slice(&tracks)));
    let t = tx.clone();
    main_window.on_clicked(move |i| t.send(RunnerMessage::PlayTrack(i as usize)).unwrap());
    let t = tx.clone();
    main_window.on_play_previous(move || t.send(RunnerMessage::PlayPrevious).unwrap());
    let t = tx.clone();
    main_window.on_toggle_pause(move || t.send(RunnerMessage::TogglePause).unwrap());
    let t = tx.clone();
    main_window.on_play_next(move || t.send(RunnerMessage::PlayNext).unwrap());
    let t = tx.clone();
    main_window.on_seek(move |time| {
        t.send(RunnerMessage::Seek(RunnerSeek::Absolute(time as f64)))
            .unwrap()
    });
    let t = tx.clone();
    main_window
        .on_set_volume(move |volume| t.send(RunnerMessage::SetVolume(volume as f64)).unwrap());
    let (tx_searching, rx_searching) = flume::unbounded();
    main_window.on_searching(move |searching| tx_searching.send(searching.to_string()).unwrap());
    let window = main_window.as_weak();

    let updater = tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        let mut searching = String::new();
        let mut old_index = usize::MAX;
        let mut loaded = 0;
        loop {
            interval.tick().await;
            let guard = runner.read().await;
            let mut index = guard.index();
            if index > len {
                index = 0;
            }
            let playback = guard.playback();
            let time = guard.time();
            let length = time.length;
            let time_float = time.position;
            let volume = guard.volume();
            let position = time.format_pos();

            let mut new_loaded = false;
            while let Ok(track_data) = rx_l.try_recv() {
                let index = track_data.index as usize;
                tracks[index] = track_data;
                new_loaded = true;
                loaded += 1;
            }
            let progress = loaded as f64 / tracks.len() as f64;
            let mut playing_track = None;
            if old_index != index || new_loaded {
                playing_track = Some(tracks[index].clone());
                old_index = index;
            }

            let mut updated_search = false;
            while let Ok(search_string) = rx_searching.try_recv() {
                searching = search_string;
                updated_search = true;
            }

            let mut t = vec![];

            let is_searching = !searching.is_empty();

            if new_loaded || updated_search {
                t = tracks.clone();
            }

            if is_searching && (updated_search || new_loaded) {
                t = t
                    .into_iter()
                    .filter(|track| {
                        let search = searching.to_lowercase();
                        track.title.to_lowercase().contains(&search)
                            || track.artist.to_lowercase().contains(&search)
                    })
                    .collect();
            }

            window
                .upgrade_in_event_loop(move |window| {
                    window.set_playing(index as i32);
                    window.set_position_time(position.into());
                    window.set_time(time_float as f32);
                    window.set_length(length as f32);
                    window.set_playback(playback);
                    window.set_volume(volume as f32);

                    if let Some(playing_track) = playing_track {
                        window.set_playing_track(playing_track);
                    }

                    if new_loaded {
                        let progress = if progress == 1.0 {
                            0.0
                        } else {
                            progress as f32
                        };
                        window.set_progress(progress);
                    }

                    if new_loaded || updated_search {
                        window.set_tracks(VecModel::from_slice(&t));
                    }
                })
                .unwrap();
        }
    });

    tokio::task::block_in_place(|| main_window.run().unwrap());

    updater.abort();
    future.abort();
    storage.lock().unwrap().save();
}
