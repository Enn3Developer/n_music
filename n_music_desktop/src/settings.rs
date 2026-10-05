use n_music_core::library::track::ReplayGainMode;
use n_music_core::settings::{
    LibrarySettings, Options, OutputDevice, PlaybackSettings, Section, SettingsStorage,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

/// Preferences of the desktop interface.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    pub theme: Theme,
    /// A bundled language's code; `None` follows the system.
    pub locale: Option<String>,
    pub window_size: WindowSize,
    pub save_window_size: bool,
    /// Looks for a new version once a launch.
    pub check_updates: bool,
    /// The mini player stays above other windows.
    pub mini_on_top: bool,
    /// The name of one of the interface's accent colours.
    pub accent: String,
    /// Track lists use smaller covers and tighter rows.
    pub compact_rows: bool,
    /// Track table columns left out, by the names `TrackColumns` knows them by.
    pub hidden_columns: Vec<String>,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            locale: None,
            window_size: WindowSize::default(),
            save_window_size: false,
            check_updates: true,
            mini_on_top: true,
            accent: String::from("amber"),
            compact_rows: false,
            hidden_columns: Vec::new(),
        }
    }
}

impl Section for UiSettings {
    const KEY: &'static str = "desktop.ui";
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: usize,
    pub height: usize,
    /// The window was maximized; `width` and `height` are its size when it is not.
    #[serde(default)]
    pub maximized: bool,
}

impl Default for WindowSize {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 800,
            maximized: false,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl From<Theme> for i32 {
    fn from(value: Theme) -> Self {
        match value {
            Theme::System => 0,
            Theme::Light => 1,
            Theme::Dark => 2,
        }
    }
}

impl From<i32> for Theme {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Light,
            2 => Self::Dark,
            _ => Self::System,
        }
    }
}

static UI: OnceLock<Options<UiSettings>> = OnceLock::new();
static FIRST_RUN: OnceLock<bool> = OnceLock::new();
static REPLAY_GAIN: OnceLock<ReplayGainMode> = OnceLock::new();
static OUTPUT_DEVICE: OnceLock<Option<OutputDevice>> = OnceLock::new();

/// Loads the interface preferences; call once, before the interface starts.
pub fn load(storage: Arc<dyn SettingsStorage>) {
    // The library saves its sources once they are chosen, on the first run or in the settings.
    let _ = FIRST_RUN.set(storage.load(LibrarySettings::KEY).is_none());
    // The player applies its saved mode without reporting it: read it where it saves it.
    let playback = Options::<PlaybackSettings>::load(storage.clone());
    let _ = REPLAY_GAIN.set(playback.get().replay_gain);
    let _ = OUTPUT_DEVICE.set(playback.get().output_device.clone());
    let _ = UI.set(Options::load(storage));
}

/// The ReplayGain mode playback started with.
pub fn replay_gain() -> ReplayGainMode {
    REPLAY_GAIN.get().copied().unwrap_or_default()
}

/// The output device playback started with; `None` is the system default.
pub fn output_device() -> Option<OutputDevice> {
    OUTPUT_DEVICE.get().cloned().flatten()
}

/// No sources were ever chosen: the app runs for the first time.
pub fn first_run() -> bool {
    FIRST_RUN.get().copied().unwrap_or(false)
}

/// The interface preferences.
pub fn ui() -> &'static Options<UiSettings> {
    UI.get().expect("settings::load runs first")
}
