use crate::localization::{get_locale_denominator, localize};
use crate::ui::color_scheme;
use crate::{Localization, MainWindow, SettingsData};
use n_event_bus::{
    Ctx, Handle, Outbox, Registrar, RunningJob, ShutdownRequested, Subscriber, Tagged,
};
use n_music_core::jobs::settings::{
    DirectoryChosen, DirectoryJob, OpenLinkJob, PersistJob, Persisted,
};
use n_music_core::messages::*;
use n_music_core::platform::Platform;
use n_music_core::settings::Settings;
use n_music_core::{FileTrack, Theme, WindowSize};
use slint::{ComponentHandle, Weak};
use std::{any::Any, mem, path::PathBuf, sync::Arc};

enum UiChange {
    Theme(Theme),
    Locale(String),
    Path(String),
}

pub struct SettingsScene {
    window: Weak<MainWindow>,
    settings: Settings,
    platform: Arc<dyn Platform>,
    internal_dir: PathBuf,
    ui_changes: Vec<UiChange>,
    persistence: Option<RunningJob>,
    directory: Option<RunningJob>,
    dirty: bool,
    closing: bool,
    tracks: Option<Arc<Vec<FileTrack>>>,
}

impl SettingsScene {
    pub fn new(
        window: Weak<MainWindow>,
        settings: Settings,
        platform: Arc<dyn Platform>,
        internal_dir: PathBuf,
    ) -> Self {
        Self {
            window,
            settings,
            platform,
            internal_dir,
            ui_changes: vec![],
            persistence: None,
            directory: None,
            dirty: false,
            closing: false,
            tracks: None,
        }
    }
    fn persist(&mut self, ctx: &Ctx, out: &mut Outbox) {
        self.dirty = true;
        self.flush(ctx, out);
    }
    fn flush(&mut self, ctx: &Ctx, out: &mut Outbox) {
        if self.persistence.is_some() {
            return;
        }
        if self.dirty {
            self.dirty = false;
            self.persistence = Some(ctx.jobs.spawn_oneshot(PersistJob {
                settings: self.settings.clone(),
                internal_dir: self.internal_dir.clone(),
                tracks: self.tracks.take(),
            }));
        } else if self.closing {
            out.shutdown_ready();
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
        reg.on::<VolumeChanged>();
        reg.on::<ScanRequested>();
        reg.on::<CacheReady>();
        reg.on::<WindowSizeCaptured>();
        reg.on::<ShutdownRequested>();
        reg.on::<OpenLink>();
        PersistJob::subscribe(reg);
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
    fn handle(&mut self, msg: &ThemeChangeRequested, ctx: &Ctx, out: &mut Outbox) {
        if self.closing {
            return;
        }
        let Ok(theme) = Theme::try_from(msg.0) else {
            return;
        };
        self.settings.theme = theme;
        self.ui_changes.push(UiChange::Theme(theme));
        self.persist(ctx, out);
        self.apply_ui();
    }
}
impl Handle<LocaleChangeRequested> for SettingsScene {
    fn handle(&mut self, msg: &LocaleChangeRequested, ctx: &Ctx, out: &mut Outbox) {
        if self.closing {
            return;
        }
        let locale = get_locale_denominator(Some(msg.0.clone()));
        self.settings.locale = Some(locale.clone());
        self.ui_changes.push(UiChange::Locale(locale));
        self.persist(ctx, out);
        self.apply_ui();
    }
}
impl Handle<ToggleSaveWindowSize> for SettingsScene {
    fn handle(&mut self, msg: &ToggleSaveWindowSize, ctx: &Ctx, out: &mut Outbox) {
        if self.closing {
            return;
        }
        self.settings.save_window_size = msg.0;
        self.persist(ctx, out);
    }
}
impl Handle<VolumeChanged> for SettingsScene {
    fn handle(&mut self, msg: &VolumeChanged, ctx: &Ctx, out: &mut Outbox) {
        if self.closing && self.persistence.is_none() {
            return;
        }
        self.settings.volume = msg.0;
        // A pre-shutdown SetVolume can have its follow-up delivered during closing.
        if self.closing {
            self.persist(ctx, out);
        }
    }
}
impl Handle<PathChangeRequested> for SettingsScene {
    fn handle(&mut self, _: &PathChangeRequested, ctx: &Ctx, _: &mut Outbox) {
        if !self.closing && self.directory.is_none() {
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
        if path.is_empty() || self.closing {
            return;
        }
        self.settings.path = path.clone();
        self.settings.timestamp = None;
        self.tracks = None;
        self.ui_changes.push(UiChange::Path(path));
        self.persist(ctx, out);
        self.apply_ui();
        out.emit(ScanRequested { check_cache: false });
    }
}
impl Handle<ScanRequested> for SettingsScene {
    fn handle(&mut self, msg: &ScanRequested, _: &Ctx, out: &mut Outbox) {
        if !self.closing {
            out.emit(ScanLibrary {
                settings: self.settings.clone(),
                internal_dir: self.internal_dir.clone(),
                check_cache: msg.check_cache,
            });
        }
    }
}
impl Handle<CacheReady> for SettingsScene {
    fn handle(&mut self, msg: &CacheReady, ctx: &Ctx, out: &mut Outbox) {
        if self.closing || msg.path != self.settings.path {
            return;
        }
        self.settings.timestamp = msg.timestamp;
        self.tracks = Some(msg.tracks.clone());
        self.persist(ctx, out);
    }
}
impl Handle<Tagged<Persisted>> for SettingsScene {
    fn handle(&mut self, msg: &Tagged<Persisted>, ctx: &Ctx, out: &mut Outbox) {
        let Some(result) = self.persistence.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        if let Err(error) = &result.0 {
            eprintln!("Could not save settings: {error}");
        }
        self.persistence = None;
        self.flush(ctx, out);
    }
}
impl Handle<WindowSizeCaptured> for SettingsScene {
    fn handle(&mut self, msg: &WindowSizeCaptured, _: &Ctx, _: &mut Outbox) {
        if self.closing {
            return;
        }
        self.settings.window_size = if self.settings.save_window_size {
            WindowSize {
                width: msg.0.width,
                height: msg.0.height,
            }
        } else {
            WindowSize::default()
        };
    }
}
impl Handle<ShutdownRequested> for SettingsScene {
    fn handle(&mut self, _: &ShutdownRequested, ctx: &Ctx, out: &mut Outbox) {
        self.closing = true;
        self.directory = None;
        self.persist(ctx, out);
    }
}
impl Handle<OpenLink> for SettingsScene {
    fn handle(&mut self, msg: &OpenLink, ctx: &Ctx, _: &mut Outbox) {
        if self.closing {
            return;
        }
        ctx.jobs
            .spawn_detached(OpenLinkJob(self.platform.clone(), msg.0.clone()));
    }
}
