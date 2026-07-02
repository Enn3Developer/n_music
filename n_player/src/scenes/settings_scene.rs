use crate::localization::{get_locale_denominator, localize};
use crate::messages::{
    LocaleChangeRequested, PathChangeRequested, ScanRequested, ThemeChangeRequested,
    ToggleSaveWindowSize, VolumeChanged,
};
use crate::platform::Platform;
use crate::settings::Settings;
use crate::{Localization, MainWindow, SettingsData, Theme};
use n_event_bus::{Ctx, EventWriter, Handle, Outbox, Registrar, Scene, Subscriber, UiPatch};
use slint::{ComponentHandle, Weak};
use std::any::Any;
use std::mem;
use std::sync::Arc;
use tokio::sync::RwLock;

enum UiChange {
    Theme(Theme),
    Locale(String),
}

/// Mirrors ui/scenes/settings.slint's `Settings` component: theme, locale,
/// window-size persistence and music-path changes.
pub struct SettingsScene {
    window: Weak<MainWindow>,
    settings: Arc<RwLock<Settings>>,
    platform: Arc<dyn Platform>,
    writer: EventWriter,
    ui_changes: Vec<UiChange>,
}

impl SettingsScene {
    pub fn new(
        window: Weak<MainWindow>,
        settings: Arc<RwLock<Settings>>,
        platform: Arc<dyn Platform>,
        writer: EventWriter,
    ) -> Self {
        Self {
            window,
            settings,
            platform,
            writer,
            ui_changes: vec![],
        }
    }

    /// Applies `change` to the settings and persists them, off the dispatch thread.
    fn update_settings(&self, change: impl FnOnce(&mut Settings) + Send + 'static) {
        let settings = self.settings.clone();
        let platform = self.platform.clone();
        tokio::spawn(async move {
            let internal_dir = platform.internal_dir().await;
            let mut settings = settings.write().await;
            change(&mut settings);
            settings.save(internal_dir).await;
        });
    }
}

impl Subscriber for SettingsScene {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<ThemeChangeRequested>();
        reg.on::<LocaleChangeRequested>();
        reg.on::<ToggleSaveWindowSize>();
        reg.on::<PathChangeRequested>();
        reg.on::<VolumeChanged>();
    }
}

impl Scene for SettingsScene {
    fn on_mount(&mut self, _ctx: &Ctx, out: &mut Outbox) {
        // The startup library scan the old run_app sent eagerly.
        out.emit(ScanRequested { check_cache: true });
    }

    fn sync(&mut self, _ctx: &Ctx) -> Option<UiPatch> {
        let ui_changes = mem::take(&mut self.ui_changes);
        if ui_changes.is_empty() {
            return None;
        }
        let window = self.window.clone();
        Some(Box::new(move || {
            let Some(window) = window.upgrade() else {
                return;
            };
            for change in ui_changes {
                match change {
                    UiChange::Theme(theme) => {
                        window
                            .global::<SettingsData>()
                            .set_color_scheme(theme.into());
                    }
                    UiChange::Locale(denominator) => {
                        localize(Some(denominator), window.global::<Localization>());
                    }
                }
            }
        }))
    }
}

impl Handle<ThemeChangeRequested> for SettingsScene {
    fn handle(&mut self, msg: &ThemeChangeRequested, _ctx: &Ctx, _out: &mut Outbox) {
        let Ok(theme) = Theme::try_from(msg.0) else {
            return;
        };
        self.ui_changes.push(UiChange::Theme(theme));
        self.update_settings(move |settings| settings.theme = theme);
    }
}

impl Handle<LocaleChangeRequested> for SettingsScene {
    fn handle(&mut self, msg: &LocaleChangeRequested, _ctx: &Ctx, _out: &mut Outbox) {
        let denominator = get_locale_denominator(Some(msg.0.clone()));
        self.ui_changes.push(UiChange::Locale(denominator.clone()));
        self.update_settings(move |settings| settings.locale = Some(denominator));
    }
}

impl Handle<ToggleSaveWindowSize> for SettingsScene {
    fn handle(&mut self, msg: &ToggleSaveWindowSize, _ctx: &Ctx, _out: &mut Outbox) {
        let save_window_size = msg.0;
        let settings = self.settings.clone();
        tokio::spawn(async move {
            settings.write().await.save_window_size = save_window_size;
        });
    }
}

impl Handle<PathChangeRequested> for SettingsScene {
    fn handle(&mut self, _msg: &PathChangeRequested, _ctx: &Ctx, _out: &mut Outbox) {
        let settings = self.settings.clone();
        let platform = self.platform.clone();
        let writer = self.writer.clone();
        tokio::spawn(async move {
            let path = platform
                .ask_music_dir()
                .await
                .to_str()
                .unwrap_or_default()
                .to_string();
            let internal_dir = platform.internal_dir().await;
            {
                let mut settings = settings.write().await;
                settings.path = path;
                settings.save(internal_dir).await;
            }
            writer.emit(ScanRequested { check_cache: false });
        });
    }
}

/// Keeps the in-memory settings volume current so the exit-time save persists it.
impl Handle<VolumeChanged> for SettingsScene {
    fn handle(&mut self, msg: &VolumeChanged, _ctx: &Ctx, _out: &mut Outbox) {
        let volume = msg.0;
        let settings = self.settings.clone();
        tokio::spawn(async move {
            settings.write().await.volume = volume;
        });
    }
}
