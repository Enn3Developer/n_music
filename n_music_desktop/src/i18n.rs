//! Interface strings in the bundled languages; English fills what a language lacks.

use std::collections::BTreeMap;

include!(concat!(env!("OUT_DIR"), "/locales.rs"));

const ENGLISH: &str = "en";

/// The bundled languages as (code, name), by name.
pub fn languages() -> Vec<(&'static str, &'static str)> {
    let mut languages: Vec<_> = LOCALES
        .iter()
        .map(|&(code, name, _)| (code, name))
        .collect();
    languages.sort_by_key(|&(_, name)| name.to_lowercase());
    languages
}

/// The language to show: `saved`, else the system's, else English.
pub fn resolve(saved: Option<&str>) -> &'static str {
    let system = sys_locale::get_locale().unwrap_or_else(|| {
        log::warn!("Could not detect the system locale; using English");
        ENGLISH.into()
    });
    let wanted = saved.unwrap_or_else(|| system.split(['-', '_']).next().unwrap_or(ENGLISH));
    LOCALES
        .iter()
        .find(|&&(code, _, _)| code == wanted)
        .map_or(ENGLISH, |&(code, _, _)| code)
}

fn parse(code: &str) -> BTreeMap<String, String> {
    let Some(&(_, _, json)) = LOCALES.iter().find(|&&(other, _, _)| other == code) else {
        return BTreeMap::new();
    };
    serde_json::from_str(json)
        .unwrap_or_else(|error| panic!("Invalid bundled language {code:?}: {error}"))
}

/// The strings of `code`, by key.
pub fn strings(code: &str) -> BTreeMap<String, String> {
    let mut strings = parse(ENGLISH);
    strings.extend(parse(code));
    strings
}
