use crate::app::library_label;
use crate::localization::{get_locale_denominator, localize};
use crate::settings::UiSettings;
use crate::ui::color_scheme;
use crate::{Localization, MainWindow, SettingsData};
use n_event_bus::{
    Ctx, Handle, Outbox, Registrar, RunningJob, ShutdownRequested, Subscriber, Tagged,
};
use n_music_core::jobs::settings::{DirectoryChosen, DirectoryJob, OpenLinkJob};
use n_music_core::messages::*;
use n_music_core::platform::Platform;
use n_music_core::settings::{LibrarySettings, Options};
use n_music_core::source::Locator;
use n_music_core::{Theme, WindowSize};
use slint::{ComponentHandle, Weak};
use std::{any::Any, mem, sync::Arc};

enum UiChange {
    Theme(Theme),
    Locale(String),
    Path(String),
}

pub struct SettingsScene {
    window: Weak<MainWindow>,
    ui: Options<UiSettings>,
    library: Options<LibrarySettings>,
    platform: Arc<dyn Platform>,
    ui_changes: Vec<UiChange>,
    directory: Option<RunningJob>,
}

impl SettingsScene {
    pub fn new(
        window: Weak<MainWindow>,
        ui: Options<UiSettings>,
        library: Options<LibrarySettings>,
        platform: Arc<dyn Platform>,
    ) -> Self {
        Self {
            window,
            ui,
            library,
            platform,
            ui_changes: vec![],
            directory: None,
        }
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
        reg.on::<WindowSizeCaptured>();
        reg.on::<ShutdownRequested>();
        reg.on::<OpenLink>();
        DirectoryJob::subscribe(reg);
    }
}
impl SettingsScene {
    fn apply_ui(&mut self) {
        let changes = mem::take(&mut self.ui_changes);
        if changes.is_empty() {
            return;
        }
        let window = self.window.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(window) = window.upgrade() else {
                return;
            };
            for change in changes {
                match change {
                    UiChange::Theme(theme) => window
                        .global::<SettingsData>()
                        .set_color_scheme(color_scheme(theme)),
                    UiChange::Locale(locale) => {
                        localize(Some(locale), window.global::<Localization>())
                    }
                    UiChange::Path(path) => window
                        .global::<SettingsData>()
                        .set_current_path(path.into()),
                }
            }
        });
    }
}
impl Handle<ThemeChangeRequested> for SettingsScene {
    fn handle(&mut self, msg: &ThemeChangeRequested, ctx: &Ctx, _: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let Ok(theme) = Theme::try_from(msg.0) else {
            return;
        };
        self.ui.update(|ui| ui.theme = theme);
        self.ui_changes.push(UiChange::Theme(theme));
        self.apply_ui();
    }
}
impl Handle<LocaleChangeRequested> for SettingsScene {
    fn handle(&mut self, msg: &LocaleChangeRequested, ctx: &Ctx, _: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let locale = get_locale_denominator(Some(msg.0.clone()));
        self.ui.update(|ui| ui.locale = Some(locale.clone()));
        self.ui_changes.push(UiChange::Locale(locale));
        self.apply_ui();
    }
}
impl Handle<ToggleSaveWindowSize> for SettingsScene {
    fn handle(&mut self, msg: &ToggleSaveWindowSize, ctx: &Ctx, _: &mut Outbox) {
        if !ctx.shutting_down {
            self.ui.update(|ui| ui.save_window_size = msg.0);
        }
    }
}
impl Handle<PathChangeRequested> for SettingsScene {
    fn handle(&mut self, _: &PathChangeRequested, ctx: &Ctx, _: &mut Outbox) {
        if !ctx.shutting_down && self.directory.is_none() {
            self.directory = Some(ctx.jobs.spawn_oneshot(DirectoryJob(self.platform.clone())));
        }
    }
}
impl Handle<Tagged<DirectoryChosen>> for SettingsScene {
    fn handle(&mut self, msg: &Tagged<DirectoryChosen>, ctx: &Ctx, out: &mut Outbox) {
        let Some(chosen) = self.directory.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        let path = chosen.0.to_string_lossy().into_owned();
        self.directory = None;
        if path.is_empty() || ctx.shutting_down {
            return;
        }
        // Picking a folder replaces the libraries until there is UI to manage several.
        let libraries = vec![Locator::library_root(&path)];
        self.ui_changes
            .push(UiChange::Path(library_label(&libraries)));
        self.library.update(|library| library.libraries = libraries);
        self.apply_ui();
        out.emit(ScanRequested { check_cache: true });
    }
}
impl Handle<WindowSizeCaptured> for SettingsScene {
    fn handle(&mut self, msg: &WindowSizeCaptured, _: &Ctx, _: &mut Outbox) {
        self.ui.update(|ui| {
            ui.window_size = if ui.save_window_size {
                msg.0
            } else {
                WindowSize::default()
            };
        });
    }
}
impl Handle<ShutdownRequested> for SettingsScene {
    fn handle(&mut self, _: &ShutdownRequested, _: &Ctx, out: &mut Outbox) {
        self.directory = None;
        out.shutdown_ready();
    }
}
impl Handle<OpenLink> for SettingsScene {
    fn handle(&mut self, msg: &OpenLink, ctx: &Ctx, _: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        ctx.jobs
            .spawn_detached(OpenLinkJob(self.platform.clone(), msg.0.clone()));
    }
}
