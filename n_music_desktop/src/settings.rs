use n_music_core::settings::{Options, Section, SettingsStorage};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

/// Preferences of the desktop interface.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    pub theme: Theme,
    /// A bundled language's code; `None` follows the system.
    pub locale: Option<String>,
    pub window_size: WindowSize,
    pub save_window_size: bool,
}

impl Section for UiSettings {
    const KEY: &'static str = "desktop.ui";
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: usize,
    pub height: usize,
}

impl Default for WindowSize {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 800,
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

/// Loads the interface preferences; call once, before the interface starts.
pub fn load(storage: Arc<dyn SettingsStorage>) {
    let _ = UI.set(Options::load(storage));
}

/// The interface preferences.
pub fn ui() -> &'static Options<UiSettings> {
    UI.get().expect("settings::load runs first")
}
