//! The desktop interface's preferences, kept in the settings section `desktop.ui` exactly as
//! the Qt app keeps them (`n_music_desktop/src/settings.rs`), so the apps share them.

use serde_json::{json, Map, Value};

/// The settings section of [`DesktopUi`].
pub const KEY: &str = "desktop.ui";

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum UiTheme {
    System,
    Light,
    Dark,
}

/// The preferences of the desktop interface, as `desktop.ui` keeps them.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct DesktopUi {
    pub theme: UiTheme,
    /// A bundled language's code, like `it`; `None` follows the system.
    pub locale: Option<String>,
    /// The window's size when not maximized, saved as it closes while `save_window_size`.
    pub window_width: u32,
    pub window_height: u32,
    pub window_maximized: bool,
    pub save_window_size: bool,
    /// Looks for a new version once a launch.
    pub check_updates: bool,
    /// The mini player stays above other windows.
    pub mini_on_top: bool,
    /// One of `amber`, `green`, `teal`, `blue`, `violet` and `rose`.
    pub accent: String,
    /// Track lists use smaller covers and tighter rows.
    pub compact_rows: bool,
    /// Things change in place rather than travel across the screen.
    pub reduce_motion: bool,
    /// Track table columns left out: `number`, `album`, `genre`, `year`, `added`, `plays`,
    /// `lastPlayed`, `format`, `time`.
    pub hidden_columns: Vec<String>,
    /// The folder picker lists hidden folders too.
    pub show_hidden_folders: bool,
}

impl Default for DesktopUi {
    fn default() -> Self {
        Self {
            theme: UiTheme::System,
            locale: None,
            window_width: 1280,
            window_height: 800,
            window_maximized: false,
            save_window_size: false,
            check_updates: true,
            mini_on_top: true,
            accent: String::from("amber"),
            compact_rows: false,
            reduce_motion: false,
            hidden_columns: vec![],
            show_hidden_folders: false,
        }
    }
}

/// The desktop interface's preferences as the Qt app starts with them.
#[uniffi::export]
pub fn default_desktop_ui() -> DesktopUi {
    DesktopUi::default()
}

impl DesktopUi {
    /// The preferences in `section`, each missing or unreadable one at its default.
    pub fn read(section: Option<&Value>) -> Self {
        let mut ui = Self::default();
        let Some(section) = section.and_then(Value::as_object) else {
            return ui;
        };
        let flag = |key: &str, value: &mut bool| {
            if let Some(read) = section.get(key).and_then(Value::as_bool) {
                *value = read;
            }
        };
        flag("save_window_size", &mut ui.save_window_size);
        flag("check_updates", &mut ui.check_updates);
        flag("mini_on_top", &mut ui.mini_on_top);
        flag("compact_rows", &mut ui.compact_rows);
        flag("reduce_motion", &mut ui.reduce_motion);
        flag("show_hidden_folders", &mut ui.show_hidden_folders);
        match section.get("theme").and_then(Value::as_str) {
            Some("Light") => ui.theme = UiTheme::Light,
            Some("Dark") => ui.theme = UiTheme::Dark,
            _ => {}
        }
        ui.locale = section
            .get("locale")
            .and_then(Value::as_str)
            .map(String::from);
        if let Some(accent) = section.get("accent").and_then(Value::as_str) {
            ui.accent = accent.to_string();
        }
        if let Some(columns) = section.get("hidden_columns").and_then(Value::as_array) {
            ui.hidden_columns = columns
                .iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect();
        }
        if let Some(size) = section.get("window_size") {
            let side = |key: &str| {
                size.get(key)
                    .and_then(Value::as_u64)
                    .and_then(|side| u32::try_from(side).ok())
            };
            if let (Some(width), Some(height)) = (side("width"), side("height")) {
                ui.window_width = width;
                ui.window_height = height;
                ui.window_maximized = size
                    .get("maximized")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            }
        }
        ui
    }

    /// `section` with these preferences, keeping what else it holds.
    pub fn write(&self, section: Option<Value>) -> Value {
        let mut map = match section {
            Some(Value::Object(map)) => map,
            _ => Map::new(),
        };
        let theme = match self.theme {
            UiTheme::System => "System",
            UiTheme::Light => "Light",
            UiTheme::Dark => "Dark",
        };
        for (key, value) in [
            ("theme", json!(theme)),
            ("locale", json!(self.locale)),
            (
                "window_size",
                json!({
                    "width": self.window_width.max(1),
                    "height": self.window_height.max(1),
                    "maximized": self.window_maximized,
                }),
            ),
            ("save_window_size", json!(self.save_window_size)),
            ("check_updates", json!(self.check_updates)),
            ("mini_on_top", json!(self.mini_on_top)),
            ("accent", json!(self.accent)),
            ("compact_rows", json!(self.compact_rows)),
            ("reduce_motion", json!(self.reduce_motion)),
            ("hidden_columns", json!(self.hidden_columns)),
            ("show_hidden_folders", json!(self.show_hidden_folders)),
        ] {
            map.insert(key.to_string(), value);
        }
        Value::Object(map)
    }
}
