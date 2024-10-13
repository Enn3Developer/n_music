#[cfg(target_os = "linux")]
use crate::bus_server::linux::MPRISBridge;
#[cfg(not(target_os = "linux"))]
use crate::bus_server::DummyServer;
use crate::localization::{get_locale_denominator, localize};
use crate::runner::{run, Runner, RunnerMessage, RunnerSeek};
use crate::settings::Settings;
use crate::{
    add_all_tracks_to_player, bus_server, get_image, AppData, FileTrack, Localization, MainWindow,
    SettingsData, Theme, TrackData, WindowSize,
};
use flume::{Receiver, Sender};
#[cfg(target_os = "linux")]
use mpris_server::Server;
use n_audio::music_track::MusicTrack;
use n_audio::queue::QueuePlayer;
use n_audio::remove_ext;
use rimage::operations::resize::{FilterType, ResizeAlg};
use slint::{ComponentHandle, VecModel};
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::sync::RwLock;
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_image::image::Image;
use zune_image::traits::OperationsTrait;
use zune_imageprocs::crop::Crop;

pub async fn run_app(
    settings: Settings,
    #[cfg(target_os = "android")] app: slint::android::AndroidApp,
) {
    let settings = Arc::new(RefCell::new(settings));

    let tmp = NamedTempFile::new().unwrap();
    let (tx, rx) = flume::unbounded();

    let mut player = QueuePlayer::new(settings.borrow().path.clone());
    add_all_tracks_to_player(&mut player, settings.borrow().path.clone()).await;
    let len = player.len();

    let runner = Arc::new(RwLock::new(Runner::new(player)));

    let r = runner.clone();
    #[cfg(target_os = "linux")]
    let tx_t = tx.clone();

    let (tx_l, rx_l) = flume::unbounded();
    let main_window = MainWindow::new().unwrap();

    localize(
        settings.borrow().locale.clone(),
        main_window.global::<Localization>(),
    );

    let check_timestamp = settings.borrow().check_timestamp().await;
    let is_cached = !check_timestamp
        && !settings.borrow().tracks.is_empty()
        && settings.borrow().tracks.len() == len;

    #[cfg(target_os = "android")]
    let a = app.clone();
    let future = tokio::spawn(async move {
        #[cfg(target_os = "linux")]
        let server = Server::new("n_music", MPRISBridge::new(r.clone(), tx_t.clone()))
            .await
            .unwrap();
        #[cfg(not(target_os = "linux"))]
        let server = DummyServer;

        let runner_future = tokio::task::spawn(run(r.clone(), rx));
        let bus_future = tokio::task::spawn(bus_server::run(server, r.clone(), tmp));
        if !is_cached {
            #[cfg(not(target_os = "android"))]
            let loader_future = tokio::task::spawn(loader(r.clone(), tx_l));
            #[cfg(target_os = "android")]
            let loader_future = {
                let app = a.clone();
                tokio::task::spawn(loader(r.clone(), tx_l, app))
            };

            let _ = tokio::join!(runner_future, bus_future, loader_future);
        } else {
            let _ = tokio::join!(runner_future, bus_future);
        }
    });

    let mut tracks = vec![];
    for i in 0..len {
        let track_path = runner.read().await.get_path_for_file(i).await.unwrap();
        if is_cached {
            let track_without_ext = remove_ext(track_path);
            if let Some(file_track) = settings
                .borrow()
                .tracks
                .iter()
                .find(|file_track| file_track.path == track_without_ext)
            {
                let mut track: TrackData = file_track.clone().into();
                track.index = i as i32;
                tracks.push(track)
            }
        } else {
            tracks.push(TrackData {
                artist: Default::default(),
                cover: Default::default(),
                time: Default::default(),
                title: remove_ext(track_path).into(),
                index: i as i32,
            });
        }
    }
    let tracks_len = tracks.len();

    let settings_data = main_window.global::<SettingsData>();
    let app_data = main_window.global::<AppData>();

    #[cfg(target_os = "android")]
    app_data.set_android(true);
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
    #[cfg(target_os = "android")]
    let a = app.clone();
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
            #[cfg(target_os = "android")]
            let app = a.clone();
            slint::spawn_local(async move {
                #[cfg(not(target_os = "android"))]
                s.borrow().save().await;
                #[cfg(target_os = "android")]
                s.borrow().save(&app).await;
            })
            .unwrap();
        });
    tokio::task::block_in_place(|| app_data.set_tracks(VecModel::from_slice(&tracks)));
    let s = settings.clone();
    let window = main_window.clone_strong();
    #[cfg(target_os = "android")]
    let a = app.clone();
    settings_data.on_change_theme_callback(move |theme_name| {
        if let Ok(theme) = Theme::try_from(theme_name) {
            s.borrow_mut().theme = theme;
            window
                .global::<SettingsData>()
                .set_color_scheme(theme.into());
            let s = s.clone();
            #[cfg(target_os = "android")]
            let app = a.clone();
            slint::spawn_local(async move {
                #[cfg(not(target_os = "android"))]
                s.borrow_mut().save().await;
                #[cfg(target_os = "android")]
                s.borrow_mut().save(&app).await;
            })
            .unwrap();
        }
    });
    let s = settings.clone();
    settings_data.on_toggle_save_window_size(move |save| s.borrow_mut().save_window_size = save);
    #[cfg(not(target_os = "android"))]
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
    let (tx_t, rx_t) = flume::unbounded();
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
                if let Some((index, file_track)) = track_data {
                    tx_t.send_async(file_track.clone()).await.unwrap();
                    tracks[index] = file_track.into();
                    tracks[index].index = index as i32;
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
    let tracks: Vec<FileTrack> = rx_t.iter().collect();
    if tracks.len() == tracks_len {
        settings.borrow_mut().tracks = tracks;
        settings.borrow_mut().save_timestamp().await;
    }
    #[cfg(not(target_os = "android"))]
    settings.borrow_mut().save().await;
    #[cfg(target_os = "android")]
    settings.borrow_mut().save(&app).await;
}
async fn loader_task(
    runner: Arc<RwLock<Runner>>,
    tx: Sender<Option<(usize, FileTrack)>>,
    rx_l: Arc<tokio::sync::Mutex<Receiver<usize>>>,
    #[cfg(target_os = "android")] app: slint::android::AndroidApp,
) {
    loop {
        if let Ok(index) = rx_l.lock().await.recv_async().await {
            if index == usize::MAX {
                if let Err(e) = tx.send_async(None).await {
                    eprintln!("error happened when signaling end of task, probably because the app was closed: {e}");
                }
                return;
            }
            if let Some(path) = runner.read().await.get_path_for_file(index).await {
                if let Ok(track) = MusicTrack::new(path.to_string_lossy().to_string()) {
                    if let Ok(Ok(meta)) =
                        tokio::task::spawn_blocking(move || track.get_meta()).await
                    {
                        let p = path.clone();
                        let image = if let Ok(image) =
                            tokio::task::spawn_blocking(move || get_image(p)).await
                        {
                            if !image.is_empty() {
                                let mut zune_image =
                                    Image::read(ZCursor::new(image), DecoderOptions::new_fast())
                                        .unwrap();
                                zune_image.convert_color(ColorSpace::RGB).unwrap();
                                let (width, height) = zune_image.dimensions();
                                if width != height {
                                    let difference = width.abs_diff(height);
                                    let min = width.min(height);
                                    let is_height = height < width;
                                    let x = if is_height { difference / 2 } else { 0 };
                                    let y = if !is_height { difference / 2 } else { 0 };
                                    tokio::task::block_in_place(|| {
                                        Crop::new(min, min, x, y).execute(&mut zune_image).unwrap()
                                    });
                                }
                                tokio::task::block_in_place(|| {
                                    rimage::operations::resize::Resize::new(
                                        128,
                                        128,
                                        ResizeAlg::Convolution(FilterType::Hamming),
                                    )
                                    .execute(&mut zune_image)
                                    .unwrap()
                                });
                                zune_image.flatten_to_u8()[0].clone()
                            } else {
                                vec![]
                            }
                        } else {
                            vec![]
                        };

                        if let Err(e) = tx
                            .send_async(Some((
                                index,
                                FileTrack {
                                    path: remove_ext(path),
                                    title: meta.title,
                                    artist: meta.artist,
                                    length: meta.time.length,
                                    image,
                                },
                            )))
                            .await
                        {
                            eprintln!("error happened during metadata transfer, probably because the app was closed: {e}");
                        }
                    }
                }
            }
        }
    }
}

async fn loader(
    runner: Arc<RwLock<Runner>>,
    tx: Sender<Option<(usize, FileTrack)>>,
    #[cfg(target_os = "android")] app: slint::android::AndroidApp,
) {
    let len = runner.read().await.len();
    let mut tasks = vec![];
    let (tx_l, rx_l) = flume::unbounded();
    let rx_l = Arc::new(tokio::sync::Mutex::new(rx_l));
    let cpus = num_cpus::get() * 4;
    for _ in 0..cpus {
        let runner = runner.clone();
        let tx = tx.clone();
        let rx_l = rx_l.clone();
        #[cfg(target_os = "android")]
        let app = app.clone();
        #[cfg(not(target_os = "android"))]
        tasks.push(tokio::task::spawn(loader_task(runner, tx, rx_l)));
        #[cfg(target_os = "android")]
        tasks.push(tokio::task::spawn(loader_task(runner, tx, rx_l, app)));
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
