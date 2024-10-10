use crate::Localization;
use serde::{Deserialize, Serialize};

include!(concat!(env!("OUT_DIR"), "/localizations.rs"));

#[derive(Debug, Deserialize, Serialize)]
pub struct Locale {
    settings: Option<String>,
    search: Option<String>,
    theme: Option<String>,
    window_size: Option<String>,
    music_path: Option<String>,
}

pub fn localize(denominator: Option<String>, localization: Localization) {
    let denominator = denominator.unwrap_or(
        sys_locale::get_locale()
            .unwrap()
            .split('-')
            .next()
            .unwrap()
            .to_string(),
    );
    let locale = get_locale(&denominator);
    let english = get_locale("en");

    localization.set_settings(
        locale
            .settings
            .as_ref()
            .unwrap_or(english.settings.as_ref().unwrap())
            .into(),
    );
    localization.set_search(
        locale
            .search
            .as_ref()
            .unwrap_or(english.search.as_ref().unwrap())
            .into(),
    );
    localization.set_theme(
        locale
            .theme
            .as_ref()
            .unwrap_or(english.theme.as_ref().unwrap())
            .into(),
    );
    localization.set_window_size(
        locale
            .window_size
            .as_ref()
            .unwrap_or(english.window_size.as_ref().unwrap())
            .into(),
    );
    localization.set_music_path(
        locale
            .music_path
            .as_ref()
            .unwrap_or(english.music_path.as_ref().unwrap())
            .into(),
    );
}

pub fn get_locale_name(denominator: &str) -> &str {
    for localization in LOCALIZATIONS {
        if denominator == localization.0 {
            return localization.1;
        }
    }
    "English"
}
