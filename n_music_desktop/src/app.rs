use crate::localization::localize;
use crate::scenes::{AppScene, SettingsScene};
use crate::ui::color_scheme;
use crate::{AppData, Localization, MainWindow, SettingsData};
use n_event_bus::{App, EventWriter, JobControl};
use n_music_core::messages::{
    LocaleChangeRequested, OpenLink, PathChangeRequested, PlayNext, PlayPrevious, PlayTrack,
    ScanRequested, SearchChanged, Seek, SetVolume, ThemeChangeRequested, TogglePause, ToggleRepeat,
    ToggleSaveWindowSize,
};
use n_music_core::platform::Platform;
use n_music_core::queue::QueuePlayer;
use n_music_core::settings::Settings;
use n_music_core::WindowSize;
use slint::ComponentHandle;
use std::sync::Arc;

pub async fn run<P: Platform + 'static>(
    settings: Settings,
    platform: P,
    writer: EventWriter,
    rx: n_event_bus::EventReceiver,
    pending: Vec<n_event_bus::Event>,
) {
    let platform: Arc<dyn Platform> = Arc::new(platform);
    let internal_dir = platform.internal_dir().await;

    let p = platform.clone();
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_panic(info);
        p.set_clipboard_text(info.to_string());
        std::process::exit(1);
    }));

    let _ = slint::set_xdg_app_id("n_music");
    let main_window = MainWindow::new().unwrap();

    setup_data(&settings, &main_window, writer.clone()).await;

    let jobs = JobControl::new(writer.clone());
    let mut app = App::new(jobs.clone());

    let mut player = QueuePlayer::new(settings.path.clone());
    player.set_volume(settings.volume as f32);
    let loop_status = player.loop_status();
    app.register_subscriber(player);
    let mut app_scene = AppScene::new(main_window.as_weak(), settings.volume, loop_status);
    app_scene.apply_ui();
    app.register_subscriber(app_scene);
    app.register_subscriber(SettingsScene::new(
        main_window.as_weak(),
        settings.clone(),
        platform.clone(),
        internal_dir,
    ));

    if let Some(media) =
        n_music_media_notification::MediaNotification::new(writer.clone(), settings.volume).await
    {
        app.register_subscriber(media);
    }

    for event in pending {
        if let n_event_bus::Event::Bus(envelope) = event {
            app.enqueue(envelope);
        }
    }
    writer.emit(ScanRequested { check_cache: true });
    let bus_task = tokio::spawn(async move { app.run_loop(rx).await });

    tokio::task::block_in_place(|| main_window.run().unwrap());

    writer.emit(n_music_core::messages::Shutdown(WindowSize {
        width: main_window.get_last_width() as usize,
        height: main_window.get_last_height() as usize,
    }));
    let _ = bus_task.await;
}

async fn setup_data(settings: &Settings, main_window: &MainWindow, writer: EventWriter) {
    localize(
        settings.locale.clone(),
        main_window.global::<Localization>(),
    );

    let settings_data = main_window.global::<SettingsData>();
    let app_data = main_window.global::<AppData>();

    app_data.set_version(env!("CARGO_PKG_VERSION").into());

    {
        settings_data.set_color_scheme(color_scheme(settings.theme));
        settings_data.set_theme(i32::from(settings.theme));
        settings_data.set_width(settings.window_size.width as f32);
        settings_data.set_height(settings.window_size.height as f32);
        settings_data.set_save_window_size(settings.save_window_size);
        settings_data.set_current_path(settings.path.clone().into());
    }

    let w = writer.clone();
    app_data.on_open_link(move |link| w.emit(OpenLink(link.into())));

    let w = writer.clone();
    main_window
        .global::<Localization>()
        .on_set_locale(move |locale_name| w.emit(LocaleChangeRequested(locale_name.into())));
    let w = writer.clone();
    settings_data.on_change_theme_callback(move |theme| w.emit(ThemeChangeRequested(theme)));
    let w = writer.clone();
    settings_data.on_toggle_save_window_size(move |save| w.emit(ToggleSaveWindowSize(save)));
    let w = writer.clone();
    settings_data.on_path(move || w.emit(PathChangeRequested));
    let w = writer.clone();
    settings_data.on_scan(move || w.emit(ScanRequested { check_cache: false }));

    let w = writer.clone();
    app_data.on_clicked(move |i| w.emit(PlayTrack(i as usize)));
    let w = writer.clone();
    app_data.on_play_previous(move || w.emit(PlayPrevious));
    let w = writer.clone();
    app_data.on_toggle_pause(move || w.emit(TogglePause));
    let w = writer.clone();
    app_data.on_toggle_repeat(move || w.emit(ToggleRepeat));
    let w = writer.clone();
    app_data.on_play_next(move || w.emit(PlayNext));
    let w = writer.clone();
    app_data.on_seek(move |time, revision| {
        w.emit(Seek::FromUi {
            position: time as f64,
            revision,
        });
    });
    let w = writer.clone();
    app_data.on_set_volume(move |volume| w.emit(SetVolume(volume as f64)));
    let w = writer.clone();
    app_data.on_searching(move |searching| w.emit(SearchChanged(searching.to_string())));
}
