use crate::localization::localize;
use crate::messages::{
    LocaleChangeRequested, OpenLink, PathChangeRequested, SearchChanged, ThemeChangeRequested,
    ToggleSaveWindowSize, WindowSizeCaptured,
};
use crate::scenes::app_scene::TrackClicked;
use crate::scenes::{AppScene, SettingsScene};
use crate::settings::{UiSettings, WindowSize};
use crate::ui::color_scheme;
use crate::{AppData, Localization, MainWindow, SettingsData};
use n_event_bus::{App, EventWriter, JobControl, ShutdownOutcome};
use n_music_core::engine::Engine;
use n_music_core::messages::{
    PlayNext, PlayPrevious, ScanRequested, Seek, SetVolume, TogglePause, ToggleRepeat,
};
use n_music_core::settings::{Options, SettingsStorage};
use n_music_core::source::{LocalProvider, Providers};
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
    storage: Arc<dyn SettingsStorage>,
    platform: crate::platform::NativePlatform,
    writer: EventWriter,
    rx: n_event_bus::EventReceiver,
    pending: Vec<n_event_bus::Event>,
) {
    let platform = Arc::new(platform);
    let ui = Options::<UiSettings>::load(storage.clone());

    let _ = slint::set_xdg_app_id("n_music");
    let main_window = MainWindow::new().expect("Failed to create the desktop Slint window");

    setup_data(&ui.get(), &main_window, writer.clone());

    let jobs = JobControl::new(writer.clone());
    let mut app = App::new(jobs.clone());

    let providers = Arc::new(Providers::default().with_local(LocalProvider));

    Engine::start(
        &mut app,
        &writer,
        storage,
        providers,
        &platform.internal_dir(),
        &platform.cache_dir(),
    );
    let mut app_scene = AppScene::new(main_window.as_weak());
    app_scene.apply_ui();
    app.register_subscriber(app_scene);
    app.register_subscriber(SettingsScene::new(
        main_window.as_weak(),
        ui,
        platform.clone(),
    ));

    if let Some(media) = n_music_media_notification::MediaNotification::new(writer.clone()) {
        app.register_subscriber(media);
    }

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

    main_window.run().expect("Desktop Slint event loop failed");

    // Also handle event-loop exits that did not originate from a window close request.
    request_shutdown(&main_window, &writer);
    if bus_thread.join().is_err() {
        log::error!("Event bus thread panicked");
    }
}

fn request_shutdown(window: &MainWindow, writer: &EventWriter) {
    log::debug!("Shutdown requested");
    writer.emit(WindowSizeCaptured(WindowSize {
        width: window.get_last_width() as usize,
        height: window.get_last_height() as usize,
    }));
    writer.shutdown();
}

fn setup_data(settings: &UiSettings, main_window: &MainWindow, writer: EventWriter) {
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
    app_data.on_clicked(move |i| w.emit(TrackClicked(i as usize)));
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
        w.emit(Seek::Tracked {
            position: time as f64,
            request: revision as u64,
        });
    });
    let w = writer.clone();
    app_data.on_set_volume(move |volume| w.emit(SetVolume(volume as f64)));
    let w = writer.clone();
    app_data.on_searching(move |searching| w.emit(SearchChanged(searching.to_string())));
}

/// Library roots as shown in the settings screen.
pub fn library_label(libraries: &[n_music_core::source::Locator]) -> String {
    libraries
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
