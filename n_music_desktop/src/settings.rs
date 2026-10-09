use n_music_core::settings::{
    LibrarySettings, Options, PlaybackSettings, Section, SettingsStorage,
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
    /// Things change in place rather than travel across the screen.
    pub reduce_motion: bool,
    /// Track table columns left out, by the names `TrackColumns` knows them by.
    pub hidden_columns: Vec<String>,
    /// The folder picker lists hidden folders too.
    pub show_hidden_folders: bool,
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
            reduce_motion: false,
            hidden_columns: Vec::new(),
            show_hidden_folders: false,
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
static PLAYBACK: OnceLock<PlaybackSettings> = OnceLock::new();

/// Loads the interface preferences; call once, before the interface starts.
pub fn load(storage: Arc<dyn SettingsStorage>) {
    // The library saves its sources once they are chosen, on the first run or in the settings.
    let _ = FIRST_RUN.set(storage.load(LibrarySettings::KEY).is_none());
    // The player applies its saved settings without reporting them: read them where it saves
    // them.
    let playback = Options::<PlaybackSettings>::load(storage.clone());
    let _ = PLAYBACK.set(playback.get().clone());
    let _ = UI.set(Options::load(storage));
}

/// The playback settings the player started with.
pub fn playback() -> &'static PlaybackSettings {
    PLAYBACK.get().expect("settings::load runs first")
}

/// No sources were ever chosen: the app runs for the first time.
pub fn first_run() -> bool {
    FIRST_RUN.get().copied().unwrap_or(false)
}

/// The interface preferences.
pub fn ui() -> &'static Options<UiSettings> {
    UI.get().expect("settings::load runs first")
}
