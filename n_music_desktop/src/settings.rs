use n_music_core::settings::Section;
use n_music_core::{Theme, WindowSize};
use serde::{Deserialize, Serialize};

/// Preferences of the desktop interface.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    pub theme: Theme,
    pub locale: Option<String>,
    pub window_size: WindowSize,
    pub save_window_size: bool,
}

impl Section for UiSettings {
    const KEY: &'static str = "desktop.ui";
}
