use crate::app::modules::loader::LoaderModule;
use crate::app::modules::updater::UpdaterModule;
use crate::app::modules::{Module, ModuleMessage};
use crate::app::{AppMessage, Messenger, Platform, Runner, Settings};
use crate::localization::{get_locale_denominator, localize};
use crate::runner::{RunnerMessage, RunnerSeek};
use crate::{AppData, Localization, MainWindow, SettingsData, Theme, TrackData};
use flume::Receiver;
use pollster::FutureExt;
use slint::platform::WindowEvent;
use slint::{ComponentHandle, VecModel, Weak};
use std::thread;

pub enum WindowMessage {
    SetPlayingIndex(u16),
    SetTimePosition(String),
    SetTime(f64),
    SetLength(f64),
    SetPlayback(bool),
    SetVolume(f64),
    SetPlayingTrack(TrackData),
    SetProgress(f64),
    SetTracks(Vec<TrackData>),
    Quit,
}

impl ModuleMessage for WindowMessage {
    fn quit() -> Self {
        Self::Quit
    }
}

pub struct WindowModule<P: crate::platform::Platform + Send + 'static> {
    platform: Platform<P>,
    settings: Settings,
    messenger: Messenger<P>,
    rx: Receiver<WindowMessage>,
}

#[async_trait::async_trait]
impl<P: crate::platform::Platform + Send + 'static> Module<P> for WindowModule<P> {
    type Message = WindowMessage;

    async fn setup(
        platform: Platform<P>,
        settings: Settings,
        _: Runner,
        messenger: Messenger<P>,
        rx: Receiver<Self::Message>,
    ) -> Self {
        Self {
            platform,
            settings,
            messenger,
            rx,
        }
    }

    fn is_async() -> bool {
        false
    }

    async fn start(&self) {
        unreachable!("WindowModule was declared as sync but the app function started it as async")
    }

    fn start_sync(&self) {
        let main_window = MainWindow::new().unwrap();
        let handle = main_window.as_weak();
        let rx = self.rx.clone();
        thread::spawn(|| thread_fn(handle, rx));
        localize(
            self.settings.lock().block_on().locale.clone(),
            main_window.global::<Localization>(),
        );
        let settings_data = main_window.global::<SettingsData>();
        let app_data = main_window.global::<AppData>();
        #[cfg(target_os = "android")]
        app_data.set_android(true);
        app_data.set_version(env!("CARGO_PKG_VERSION").into());
        {
            let settings = self.settings.lock().block_on();
            settings_data.set_color_scheme(settings.theme.into());
            settings_data.set_theme(i32::from(settings.theme));
            settings_data.set_width(settings.window_size.width as f32);
            settings_data.set_height(settings.window_size.height as f32);
            settings_data.set_save_window_size(settings.save_window_size);
            settings_data.set_current_path(settings.path.clone().into());
        }

        let p = self.platform.clone();
        app_data.on_open_link(move |link| {
            let p = p.clone();
            slint::spawn_local(async move { p.lock().await.open_link(link.into()).await }).unwrap();
        });

        let s = self.settings.clone();
        let window = main_window.clone_strong();
        let p = self.platform.clone();
        main_window
            .global::<Localization>()
            .on_set_locale(move |locale_name| {
                let denominator = get_locale_denominator(Some(locale_name.into()));
                localize(
                    Some(denominator.to_string()),
                    window.global::<Localization>(),
                );
                let s = s.clone();
                let p = p.clone();
                slint::spawn_local(async move {
                    s.lock().await.locale = Some(denominator);
                    s.lock().await.save(p.lock().await).await;
                })
                .unwrap();
            });
        let s = self.settings.clone();
        let window = main_window.clone_strong();
        let p = self.platform.clone();
        settings_data.on_change_theme_callback(move |theme_name| {
            if let Ok(theme) = Theme::try_from(theme_name) {
                window
                    .global::<SettingsData>()
                    .set_color_scheme(theme.into());
                let s = s.clone();
                let p = p.clone();
                slint::spawn_local(async move {
                    s.lock().await.theme = theme;
                    s.lock().await.save(p.lock().await).await;
                })
                .unwrap();
            }
        });
        let s = self.settings.clone();
        settings_data.on_toggle_save_window_size(move |save| {
            let s = s.clone();
            slint::spawn_local(async move {
                s.lock().await.save_window_size = save;
            })
            .unwrap();
        });
        let s = self.settings.clone();
        let p = self.platform.clone();
        let messenger = self.messenger.clone();
        settings_data.on_path(move || {
            let s = s.clone();
            let p = p.clone();
            let messenger = messenger.clone();
            slint::spawn_local(async move {
                let path = p.lock().await.ask_music_dir().await;
                messenger
                    .send_async(AppMessage::LoaderMessage(
                        <LoaderModule<P> as Module<P>>::Message::Load(path.clone()),
                    ))
                    .await
                    .unwrap();
                s.lock().await.path = path.to_str().unwrap().to_string();
                s.lock().await.save(p.lock().await).await;
            })
            .unwrap();
        });
        let messenger = self.messenger.clone();
        app_data.on_clicked(move |i| {
            messenger
                .send(AppMessage::RunnerMessage(RunnerMessage::PlayTrack(
                    i as u16,
                )))
                .unwrap()
        });
        let messenger = self.messenger.clone();
        app_data.on_play_previous(move || {
            messenger
                .send(AppMessage::RunnerMessage(RunnerMessage::PlayPrevious))
                .unwrap()
        });
        let messenger = self.messenger.clone();
        app_data.on_toggle_pause(move || {
            messenger
                .send(AppMessage::RunnerMessage(RunnerMessage::TogglePause))
                .unwrap()
        });
        let messenger = self.messenger.clone();
        app_data.on_play_next(move || {
            messenger
                .send(AppMessage::RunnerMessage(RunnerMessage::PlayNext))
                .unwrap()
        });
        let messenger = self.messenger.clone();
        app_data.on_seek(move |time| {
            messenger
                .send(AppMessage::RunnerMessage(RunnerMessage::Seek(
                    RunnerSeek::Absolute(time as f64),
                )))
                .unwrap()
        });
        let messenger = self.messenger.clone();
        app_data.on_set_volume(move |volume| {
            messenger
                .send(AppMessage::RunnerMessage(RunnerMessage::SetVolume(
                    volume as f64,
                )))
                .unwrap()
        });
        let messenger = self.messenger.clone();
        app_data.on_searching(move |searching| {
            messenger
                .send(AppMessage::UpdaterMessage(<UpdaterModule<P> as Module<
                    P,
                >>::Message::Searching(
                    searching.to_string()
                )))
                .unwrap()
        });
        let messenger = self.messenger.clone();
        app_data.on_changing(move || {
            messenger
                .send(AppMessage::UpdaterMessage(
                    <UpdaterModule<P> as Module<P>>::Message::ChangingTime,
                ))
                .unwrap()
        });
        main_window.run().unwrap();
        self.messenger.send(AppMessage::RequestQuit).unwrap();
    }
}

fn thread_fn(handle: Weak<MainWindow>, rx: Receiver<WindowMessage>) {
    while let Ok(message) = rx.recv() {
        match message {
            WindowMessage::SetPlayingIndex(index) => {
                handle
                    .upgrade_in_event_loop(move |window| {
                        let app_data = window.global::<AppData>();
                        app_data.set_playing(index.into());
                    })
                    .unwrap();
            }
            WindowMessage::SetTimePosition(position) => {
                handle
                    .upgrade_in_event_loop(move |window| {
                        let app_data = window.global::<AppData>();
                        app_data.set_position_time(position.into());
                    })
                    .unwrap();
            }
            WindowMessage::SetTime(time) => {
                handle
                    .upgrade_in_event_loop(move |window| {
                        let app_data = window.global::<AppData>();
                        app_data.set_time(time as f32);
                    })
                    .unwrap();
            }
            WindowMessage::SetLength(length) => {
                handle
                    .upgrade_in_event_loop(move |window| {
                        let app_data = window.global::<AppData>();
                        app_data.set_length(length as f32);
                    })
                    .unwrap();
            }
            WindowMessage::SetPlayback(playback) => {
                handle
                    .upgrade_in_event_loop(move |window| {
                        let app_data = window.global::<AppData>();
                        app_data.set_playback(playback);
                    })
                    .unwrap();
            }
            WindowMessage::SetVolume(volume) => {
                handle
                    .upgrade_in_event_loop(move |window| {
                        let app_data = window.global::<AppData>();
                        app_data.set_volume(volume as f32);
                    })
                    .unwrap();
            }
            WindowMessage::SetPlayingTrack(track) => {
                handle
                    .upgrade_in_event_loop(move |window| {
                        let app_data = window.global::<AppData>();
                        app_data.set_playing_track(track);
                    })
                    .unwrap();
            }
            WindowMessage::SetProgress(progress) => {
                handle
                    .upgrade_in_event_loop(move |window| {
                        let app_data = window.global::<AppData>();
                        app_data.set_progress(progress as f32);
                    })
                    .unwrap();
            }
            WindowMessage::SetTracks(tracks) => {
                handle
                    .upgrade_in_event_loop(move |window| {
                        let app_data = window.global::<AppData>();
                        app_data.set_tracks(VecModel::from_slice(&tracks));
                    })
                    .unwrap();
            }
            WindowMessage::Quit => {
                handle
                    .upgrade_in_event_loop(|window| {
                        window.window().dispatch_event(WindowEvent::CloseRequested)
                    })
                    // ignore the error, if it errors the window is probably already closed
                    .unwrap_or(());
                break;
            }
        }
    }
}
