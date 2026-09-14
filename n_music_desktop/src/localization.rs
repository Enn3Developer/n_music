use crate::Localization;
use paste::paste;
use serde::{Deserialize, Serialize};
use slint::{SharedString, VecModel};

include!(concat!(env!("OUT_DIR"), "/localizations.rs"));

macro_rules! localize {
    ($localization:ident, $locale:ident, $default_locale:ident, $name:ident) => {
        paste! {
            $localization.[<set_ $name>](
                $locale
                    .$name
                    .as_ref()
                    .unwrap_or($default_locale.$name.as_ref().unwrap())
                    .into(),
            );
        }
    };

    ($localization:ident, $locale:ident, $default_locale:ident, $($name:ident),+) => {
        $(localize!($localization, $locale, $default_locale, $name);)+
    };
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Locale {
    settings: Option<String>,
    search: Option<String>,
    theme: Option<String>,
    window_size: Option<String>,
    music_path: Option<String>,
    language: Option<String>,
    theme_system: Option<String>,
    theme_light: Option<String>,
    theme_dark: Option<String>,
    credits: Option<String>,
    license: Option<String>,
    update_text: Option<String>,
    check_update: Option<String>,
    update: Option<String>,
    rescan: Option<String>,
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

    let mut localizations = LOCALIZATIONS
        .iter()
        .map(|(_, name)| name.to_string())
        .map(|name| name.into())
        .collect::<Vec<SharedString>>();
    localizations.sort();
    localization.set_localizations(VecModel::from_slice(&localizations));
    localization.set_current_locale(get_locale_name(Some(&denominator)).into());

    localize!(
        localization,
        locale,
        english,
        settings,
        search,
        theme,
        window_size,
        music_path,
        language,
        theme_system,
        theme_light,
        theme_dark,
        credits,
        license,
        update_text,
        check_update,
        update,
        rescan
    );
}

pub fn get_locale_name(denominator: Option<&str>) -> &str {
    if let Some(denominator) = denominator {
        for localization in LOCALIZATIONS {
            if denominator == localization.0 {
                return localization.1;
            }
        }
    }
    "English"
}

pub fn get_locale_denominator(name: Option<String>) -> String {
    if let Some(name) = name.as_ref() {
        for localization in LOCALIZATIONS {
            if name == localization.1 {
                return localization.0.to_string();
            }
        }
    }
    "en".to_string()
}
