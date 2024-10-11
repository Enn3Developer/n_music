#[cfg(target_os = "linux")]
use crate::bus_server::linux::MPRISBridge;
#[cfg(not(target_os = "linux"))]
use crate::bus_server::DummyServer;
use crate::localization::{get_locale_denominator, localize};
use crate::runner::{run, Runner, RunnerMessage, RunnerSeek};
use crate::settings::Settings;
use crate::{
    add_all_tracks_to_player, bus_server, get_image, AppData, Localization, MainWindow,
    SettingsData, Theme, TrackData, WindowSize,
};
use flume::{Receiver, Sender};
use image::imageops::FilterType;
use image::ImageFormat;
#[cfg(target_os = "linux")]
use mpris_server::Server;
use n_audio::music_track::MusicTrack;
use n_audio::queue::QueuePlayer;
use n_audio::remove_ext;
use slint::{ComponentHandle, VecModel};
use std::cell::RefCell;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::sync::RwLock;

pub async fn run_app() {
    let settings = Arc::new(RefCell::new(Settings::read_saved().await));

    let tmp = NamedTempFile::new().unwrap();
    let (tx, rx) = flume::unbounded();

    let mut player = QueuePlayer::new(settings.borrow().path.clone());
    add_all_tracks_to_player(&mut player, settings.borrow().path.clone()).await;
    let len = player.len();

    let runner = Arc::new(RwLock::new(Runner::new(player)));

    let r = runner.clone();
    let tx_t = tx.clone();

    let (tx_l, rx_l) = flume::unbounded();
    let main_window = MainWindow::new().unwrap();

    localize(
        settings.borrow().locale.clone(),
        main_window.global::<Localization>(),
    );

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

    let settings_data = main_window.global::<SettingsData>();
    let app_data = main_window.global::<AppData>();

    app_data.set_version(env!("CARGO_PKG_VERSION").into());

    settings_data.set_color_scheme(settings.borrow().theme.into());
    settings_data.set_theme(i32::from(settings.borrow().theme));
    settings_data.set_width(settings.borrow().window_size.width as f32);
    settings_data.set_height(settings.borrow().window_size.height as f32);
    settings_data.set_save_window_size(settings.borrow().save_window_size);
    settings_data.set_current_path(settings.borrow().path.clone().into());

    app_data.on_open_link(move |link| open::that(link.as_str()).unwrap());
    let s = settings.clone();
    let window = main_window.clone_strong();
    main_window
        .global::<Localization>()
        .on_set_locale(move |locale_name| {
            let denominator = get_locale_denominator(Some(&locale_name));
            s.borrow_mut().locale = Some(denominator.to_string());
            localize(
                Some(denominator.to_string()),
                window.global::<Localization>(),
            );
            let s = s.clone();
            slint::spawn_local(async move {
                s.borrow().save().await;
            })
            .unwrap();
        });
    tokio::task::block_in_place(|| app_data.set_tracks(VecModel::from_slice(&tracks)));
    let s = settings.clone();
    let window = main_window.clone_strong();
    settings_data.on_change_theme_callback(move |theme_name| {
        if let Ok(theme) = Theme::try_from(theme_name) {
            s.borrow_mut().theme = theme;
            window
                .global::<SettingsData>()
                .set_color_scheme(theme.into());
            let s = s.clone();
            slint::spawn_local(async move {
                s.borrow_mut().save().await;
            })
            .unwrap();
        }
    });
    let s = settings.clone();
    settings_data.on_toggle_save_window_size(move |save| s.borrow_mut().save_window_size = save);
    let window = main_window.as_weak();
    #[cfg(not(target_os = "android"))]
    settings_data.on_path(move || {
        let window = window.clone();
        slint::spawn_local(async move {
            if let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await {
                window
                    .upgrade_in_event_loop(move |window| {
                        window
                            .global::<SettingsData>()
                            .invoke_set_path(folder.path().to_string_lossy().to_string().into())
                    })
                    .unwrap();
            }
        })
        .unwrap();
    });
    let s = settings.clone();
    settings_data.on_set_path_callback(move |path| {
        s.borrow_mut().path = path.clone().into();
    });
    let t = tx.clone();
    app_data.on_clicked(move |i| t.send(RunnerMessage::PlayTrack(i as usize)).unwrap());
    let t = tx.clone();
    app_data.on_play_previous(move || t.send(RunnerMessage::PlayPrevious).unwrap());
    let t = tx.clone();
    app_data.on_toggle_pause(move || t.send(RunnerMessage::TogglePause).unwrap());
    let t = tx.clone();
    app_data.on_play_next(move || t.send(RunnerMessage::PlayNext).unwrap());
    let t = tx.clone();
    app_data.on_seek(move |time| {
        t.send(RunnerMessage::Seek(RunnerSeek::Absolute(time as f64)))
            .unwrap()
    });
    let t = tx.clone();
    app_data.on_set_volume(move |volume| t.send(RunnerMessage::SetVolume(volume as f64)).unwrap());
    let (tx_searching, rx_searching) = flume::unbounded();
    app_data.on_searching(move |searching| tx_searching.send(searching.to_string()).unwrap());
    let window = main_window.as_weak();
    let r = runner.clone();
    let updater = tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        let mut searching = String::new();
        let mut old_index = usize::MAX;
        let mut loaded = 0;
        let threshold = num_cpus::get() * 4;
        loop {
            interval.tick().await;
            let guard = r.read().await;
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
                if let Some(track_data) = track_data {
                    let index = track_data.index as usize;
                    tracks[index] = track_data;
                    loaded += 1;
                    if loaded % threshold == 0 {
                        new_loaded = true;
                    }
                } else {
                    new_loaded = true;
                }
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
                    let app_data = window.global::<AppData>();
                    app_data.set_playing(index as i32);
                    app_data.set_position_time(position.into());
                    app_data.set_time(time_float as f32);
                    app_data.set_length(length as f32);
                    app_data.set_playback(playback);
                    app_data.set_volume(volume as f32);

                    if let Some(playing_track) = playing_track {
                        app_data.set_playing_track(playing_track);
                    }

                    if new_loaded {
                        let progress = if progress == 1.0 {
                            0.0
                        } else {
                            progress as f32
                        };
                        app_data.set_progress(progress);
                    }

                    if new_loaded || updated_search {
                        app_data.set_tracks(VecModel::from_slice(&t));
                    }
                })
                .unwrap();
        }
    });

    tokio::task::block_in_place(|| main_window.run().unwrap());
    settings.borrow_mut().volume = runner.read().await.volume();
    if settings.borrow().save_window_size {
        let width = main_window.get_last_width() as usize;
        let height = main_window.get_last_height() as usize;
        settings.borrow_mut().window_size = WindowSize { width, height };
    } else {
        settings.borrow_mut().window_size = WindowSize::default();
    }

    updater.abort();
    future.abort();
    settings.borrow_mut().save().await;
}
async fn loader_task(
    runner: Arc<RwLock<Runner>>,
    tx: Sender<Option<TrackData>>,
    rx_l: Arc<tokio::sync::Mutex<Receiver<usize>>>,
) {
    loop {
        if let Ok(index) = rx_l.lock().await.recv_async().await {
            if index == usize::MAX {
                if let Err(e) = tx.send_async(None).await {
                    eprintln!("error happened when signaling end of task, probably because the app was closed: {e}");
                }
                return;
            }
            let path = runner.read().await.get_path_for_file(index).await;
            if let Ok(track) = MusicTrack::new(path.to_string_lossy().to_string()) {
                if let Ok(Ok(meta)) = tokio::task::spawn_blocking(move || track.get_meta()).await {
                    let p = path.clone();
                    let image_path = if let Ok(mut image) =
                        tokio::task::spawn_blocking(move || get_image(p)).await
                    {
                        if !image.is_empty() {
                            if let Err(e) = image::load_from_memory(&image)
                                .unwrap()
                                .resize_to_fill(128, 128, FilterType::Lanczos3)
                                .to_rgb8()
                                .write_to(&mut Cursor::new(&mut image), ImageFormat::Jpeg)
                            {
                                eprintln!(
                                    "error happened during image resizing and conversion: {e}"
                                );
                            }

                            if cfg!(not(target_os = "android")) {
                                let images_dir = Settings::app_dir().join("images");
                                if !images_dir.exists() {
                                    if let Err(e) =
                                        tokio::fs::create_dir(images_dir.as_path()).await
                                    {
                                        eprintln!("error happened during dir creation: {e}");
                                    }
                                }
                                let path = images_dir.join(format!("{}.jpg", remove_ext(path)));
                                if let Err(e) = tokio::fs::write(path.as_path(), image).await {
                                    eprintln!("error happened during image writing: {e}");
                                }
                                path
                            } else {
                                PathBuf::new().join("thisdoesntexistsodontworryaboutit")
                            }
                        } else {
                            PathBuf::new().join("thisdoesntexistsodontworryaboutit")
                        }
                    } else {
                        PathBuf::new().join("thisdoesntexistsodontworryaboutit")
                    };

                    if let Err(e) = tx
                        .send_async(Some(TrackData {
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
                        }))
                        .await
                    {
                        eprintln!("error happened during metadata transfer, probably because the app was closed: {e}");
                    }
                }
            }
        }
    }
}

async fn loader(runner: Arc<RwLock<Runner>>, tx: Sender<Option<TrackData>>) {
    let len = runner.read().await.len();
    let mut tasks = vec![];
    let (tx_l, rx_l) = flume::unbounded();
    let rx_l = Arc::new(tokio::sync::Mutex::new(rx_l));
    let cpus = num_cpus::get() * 2;
    for _ in 0..cpus {
        let runner = runner.clone();
        let tx = tx.clone();
        let rx_l = rx_l.clone();
        tasks.push(tokio::task::spawn(loader_task(runner, tx, rx_l)));
    }
    for i in 0..len {
        tx_l.send_async(i).await.unwrap();
    }
    for _ in 0..cpus {
        tx_l.send_async(usize::MAX).await.unwrap();
    }
    for task in tasks {
        task.await.unwrap();
    }
}
