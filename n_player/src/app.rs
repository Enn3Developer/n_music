use crate::localization::localize;
use crate::messages::{
    LocaleChangeRequested, PathChangeRequested, PlayNext, PlayPrevious, PlayTrack, Seek,
    SearchChanged, SetVolume, ThemeChangeRequested, TogglePause, ToggleSaveWindowSize,
    ViewportChanging,
};
use crate::platform::Platform;
use crate::playback::PlaybackEngine;
use crate::scenes::{AppScene, SettingsScene};
use crate::{AppData, Localization, MainWindow, SettingsData, WindowSize};
use n_audio::queue::QueuePlayer;
use n_event_bus::{spawn_ticker, App, EventWriter, JobControl, UiPatch, UiThread};
use slint::ComponentHandle;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub type Settings = Arc<RwLock<crate::settings::Settings>>;

/// The one seam between n_event_bus and Slint: applies each dispatch cycle's
/// patches in a single event-loop hop.
struct SlintUi;

impl UiThread for SlintUi {
    fn apply(&self, patches: Vec<UiPatch>) {
        let _ = slint::invoke_from_event_loop(move || {
            for patch in patches {
                patch();
            }
        });
    }
}

pub async fn run_app<P: Platform + 'static>(settings: crate::settings::Settings, platform: P) {
    let platform: Arc<dyn Platform> = Arc::new(platform);
    let settings: Settings = Arc::new(RwLock::new(settings));

    let p = platform.clone();
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_panic(info);
        p.set_clipboard_text(info.to_string());
        std::process::exit(1);
    }));

    #[cfg(target_os = "linux")]
    let _ = slint::set_xdg_app_id("n_music");
    let main_window = MainWindow::new().unwrap();

    let (tx, rx) = flume::unbounded();
    let writer = EventWriter::new(tx);

    setup_data(
        settings.clone(),
        platform.clone(),
        &main_window,
        writer.clone(),
    )
    .await;

    let jobs = JobControl::new(writer.clone());
    let mut app = App::new(jobs.clone(), Box::new(SlintUi));

    let path = settings.read().await.path.clone();
    app.register_subscriber(PlaybackEngine::new(QueuePlayer::new(path)));
    app.register_scene(AppScene::new(
        main_window.as_weak(),
        settings.clone(),
        platform.clone(),
    ));
    app.register_scene(SettingsScene::new(
        main_window.as_weak(),
        settings.clone(),
        platform.clone(),
        writer.clone(),
    ));
    #[cfg(target_os = "linux")]
    if let Some(bridge) = crate::bridges::mpris::MprisBridge::new(writer.clone()).await {
        app.register_subscriber(bridge);
    }
    #[cfg(target_os = "android")]
    {
        let (jvm, callback) = platform.jni_handles();
        app.register_subscriber(crate::bridges::android::AndroidBridge::new(jvm, callback));
        jobs.spawn_detached(crate::bridges::android::AndroidEventJob);
    }

    spawn_ticker(writer.clone(), Duration::from_millis(250));
    let bus_task = tokio::spawn(async move { app.run_loop(rx).await });

    tokio::task::block_in_place(|| main_window.run().unwrap());

    bus_task.abort();

    if settings.read().await.save_window_size {
        let width = main_window.get_last_width() as usize;
        let height = main_window.get_last_height() as usize;
        settings.write().await.window_size = WindowSize { width, height };
    } else {
        settings.write().await.window_size = WindowSize::default();
    }
    settings
        .read()
        .await
        .save(platform.internal_dir().await)
        .await;
}

/// Populates the globals' initial values and wires every Slint callback to a
/// typed bus message (plus the one bus-less case, open_link).
async fn setup_data(
    settings: Settings,
    platform: Arc<dyn Platform>,
    main_window: &MainWindow,
    writer: EventWriter,
) {
    localize(
        settings.read().await.locale.clone(),
        main_window.global::<Localization>(),
    );

    let settings_data = main_window.global::<SettingsData>();
    let app_data = main_window.global::<AppData>();

    #[cfg(target_os = "android")]
    app_data.set_android(true);
    app_data.set_version(env!("CARGO_PKG_VERSION").into());

    {
        let settings = settings.read().await;
        settings_data.set_color_scheme(settings.theme.into());
        settings_data.set_theme(i32::from(settings.theme));
        settings_data.set_width(settings.window_size.width as f32);
        settings_data.set_height(settings.window_size.height as f32);
        settings_data.set_save_window_size(settings.save_window_size);
        settings_data.set_current_path(settings.path.clone().into());
    }

    // Stateless fire-and-forget platform call; no reason to route it through the bus.
    app_data.on_open_link(move |link| {
        let platform = platform.clone();
        slint::spawn_local(async move { platform.open_link(link.into()).await }).unwrap();
    });

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
    settings_data.on_scan(move || w.emit(crate::messages::ScanRequested { check_cache: false }));

    let w = writer.clone();
    app_data.on_clicked(move |i| w.emit(PlayTrack(i as usize)));
    let w = writer.clone();
    app_data.on_play_previous(move || w.emit(PlayPrevious));
    let w = writer.clone();
    app_data.on_toggle_pause(move || w.emit(TogglePause));
    let w = writer.clone();
    app_data.on_play_next(move || w.emit(PlayNext));
    let w = writer.clone();
    app_data.on_seek(move |time| w.emit(Seek::Absolute(time as f64)));
    let w = writer.clone();
    app_data.on_set_volume(move |volume| w.emit(SetVolume(volume as f64)));
    let w = writer.clone();
    app_data.on_searching(move |searching| w.emit(SearchChanged(searching.to_string())));
    app_data.on_changing(move || writer.emit(ViewportChanging));
}
