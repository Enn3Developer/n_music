use crate::localization::localize;
use crate::scenes::{AppScene, SettingsScene};
use crate::ui::color_scheme;
use crate::{AppData, Localization, MainWindow, SettingsData};
use n_event_bus::{App, EventWriter, JobControl, ShutdownOutcome};
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
use std::time::Duration;

struct QuitEventLoop;

impl Drop for QuitEventLoop {
    fn drop(&mut self) {
        let _ = slint::quit_event_loop();
    }
}

pub fn run(
    settings: Settings,
    platform: crate::platform::AndroidPlatform,
    writer: EventWriter,
    rx: n_event_bus::EventReceiver,
    pending: Vec<n_event_bus::Event>,
) {
    let (jvm, callback) = platform.jni_handles();
    let platform: Arc<dyn Platform> = Arc::new(platform);
    let internal_dir = platform.internal_dir();

    let main_window = MainWindow::new().expect("Failed to create the Android Slint window");

    setup_data(&settings, &main_window, writer.clone());

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

    app.register_subscriber(crate::bridge::AndroidBridge::new(
        jvm,
        callback,
        i32::from(settings.theme),
    ));

    for event in pending {
        app.enqueue_event(event);
    }
    let close_window = main_window.as_weak();
    let close_writer = writer.clone();
    main_window.window().on_close_requested(move || {
        if let Some(window) = close_window.upgrade() {
            request_shutdown(&window, &close_writer);
        }
        slint::CloseRequestResponse::KeepWindowShown
    });
    writer.emit(ScanRequested { check_cache: true });
    let bus_thread = std::thread::Builder::new()
        .name(String::from("n_event_bus loop"))
        .spawn(move || {
            // Debug builds unwind: a failed bus must not leave an unresponsive window.
            let _quit = QuitEventLoop;
            let outcome = app.run_loop(rx, Duration::from_secs(10));
            if outcome == ShutdownOutcome::Complete {
                log::info!("Event bus shutdown complete");
            } else {
                log::error!("Event bus shutdown incomplete: {outcome:?}");
            }
            drop(app);
        })
        .expect("failed to spawn event bus thread");

    main_window.run().expect("Android Slint event loop failed");

    // Also handle event-loop exits that did not originate from a window close request.
    request_shutdown(&main_window, &writer);
    if bus_thread.join().is_err() {
        log::error!("Event bus thread panicked");
    }
}

fn request_shutdown(window: &MainWindow, writer: &EventWriter) {
    log::debug!("Shutdown requested");
    writer.emit(n_music_core::messages::WindowSizeCaptured(WindowSize {
        width: window.get_last_width() as usize,
        height: window.get_last_height() as usize,
    }));
    writer.shutdown();
}

fn setup_data(settings: &Settings, main_window: &MainWindow, writer: EventWriter) {
    localize(
        settings.locale.clone(),
        main_window.global::<Localization>(),
    );

    let settings_data = main_window.global::<SettingsData>();
    let app_data = main_window.global::<AppData>();

    app_data.set_version(env!("CARGO_PKG_VERSION").into());
    app_data.set_android(true);

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
