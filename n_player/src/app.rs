use crate::localization::localize;
use crate::messages::{
    LocaleChangeRequested, PathChangeRequested, PlayNext, PlayPrevious, PlayTrack, SearchChanged,
    Seek, SetVolume, ThemeChangeRequested, TogglePause, ToggleSaveWindowSize,
};
use crate::platform::Platform;
use crate::runner::Runner;
use crate::scenes::{AppScene, SettingsScene};
use crate::{AppData, Localization, MainWindow, SettingsData, WindowSize};
use n_event_bus::{App, EventWriter, JobControl, UiPatch, UiThread};
use slint::ComponentHandle;
use std::sync::Arc;

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
    let (writer, rx) = EventWriter::channel();
    run_app_with_events(settings, platform, writer, rx, Vec::new()).await;
}

pub async fn run_app_with_events<P: Platform + 'static>(
    settings: crate::settings::Settings,
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

    #[cfg(target_os = "linux")]
    let _ = slint::set_xdg_app_id("n_music");
    let main_window = MainWindow::new().unwrap();

    setup_data(&settings, &main_window, writer.clone()).await;

    let jobs = JobControl::new(writer.clone());
    let mut app = App::new(jobs.clone(), Box::new(SlintUi));

    let path = settings.path.clone();
    app.register_subscriber(Runner::new(path, settings.volume));
    app.register_scene(AppScene::new(main_window.as_weak(), settings.volume));
    app.register_scene(SettingsScene::new(
        main_window.as_weak(),
        settings.clone(),
        platform.clone(),
        internal_dir,
    ));
    #[cfg(target_os = "linux")]
    if let Some(bridge) =
        crate::bridges::mpris::MprisBridge::new(writer.clone(), settings.volume).await
    {
        app.register_subscriber(bridge);
    }
    #[cfg(target_os = "android")]
    {
        let (jvm, callback) = platform.jni_handles();
        app.register_subscriber(crate::bridges::android::AndroidBridge::new(jvm, callback));
    }

    for event in pending {
        if let n_event_bus::Event::Bus(envelope) = event {
            app.enqueue(envelope);
        }
    }
    let bus_task = tokio::spawn(async move { app.run_loop(rx).await });

    tokio::task::block_in_place(|| main_window.run().unwrap());

    writer.emit(crate::messages::Shutdown(WindowSize {
        width: main_window.get_last_width() as usize,
        height: main_window.get_last_height() as usize,
    }));
    let _ = bus_task.await;
}

async fn setup_data(
    settings: &crate::settings::Settings,
    main_window: &MainWindow,
    writer: EventWriter,
) {
    localize(
        settings.locale.clone(),
        main_window.global::<Localization>(),
    );

    let settings_data = main_window.global::<SettingsData>();
    let app_data = main_window.global::<AppData>();

    #[cfg(target_os = "android")]
    app_data.set_android(true);
    app_data.set_version(env!("CARGO_PKG_VERSION").into());

    {
        settings_data.set_color_scheme(settings.theme.into());
        settings_data.set_theme(i32::from(settings.theme));
        settings_data.set_width(settings.window_size.width as f32);
        settings_data.set_height(settings.window_size.height as f32);
        settings_data.set_save_window_size(settings.save_window_size);
        settings_data.set_current_path(settings.path.clone().into());
    }

    let w = writer.clone();
    app_data.on_open_link(move |link| w.emit(crate::messages::OpenLink(link.into())));

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
