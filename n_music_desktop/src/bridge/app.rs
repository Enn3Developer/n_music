//! The window's preferences and what it reports to the core.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// 0 follows the system, 1 is light, 2 is dark; saved when changed.
        #[qproperty(i32, theme)]
        /// Saved when changed.
        #[qproperty(bool, save_window_size)]
        /// Looks for a new version once a launch; saved when changed.
        #[qproperty(bool, check_updates)]
        /// The mini player stays above other windows; saved when changed.
        #[qproperty(bool, mini_on_top)]
        /// One of `Theme.accents`; saved when changed.
        #[qproperty(QString, accent)]
        /// Track lists use smaller covers and tighter rows; saved when changed.
        #[qproperty(bool, compact_rows)]
        /// Track table columns left out, see `TrackColumns.hidden`; saved when changed.
        #[qproperty(QStringList, hidden_columns)]
        #[qproperty(i32, window_width)]
        #[qproperty(i32, window_height)]
        #[qproperty(QString, version)]
        /// 0 plays tracks as mastered, 1 levels each track, 2 each album; sent when changed.
        #[qproperty(i32, replay_gain)]
        /// Where the logs are, as a URL to open.
        #[qproperty(QString, logs_folder)]
        type AppState = super::AppStateRust;

        /// Saves the window size, when it is to be remembered.
        #[qinvokable]
        fn window_closing(self: &AppState, width: i32, height: i32);

        /// Whether the window can be seen; the core sends no positions while it cannot.
        #[qinvokable]
        fn set_visible(self: &AppState, visible: bool);
    }

    impl cxx_qt::Initialize for AppState {}
}

use crate::settings::{self, Theme, WindowSize};
use crate::{bus, platform};
use core::pin::Pin;
use cxx_qt_lib::{QList, QString, QStringList, QUrl};
use n_music_core::library::track::ReplayGainMode;
use n_music_core::messages::{AppVisibilityChanged, SetReplayGain};

pub struct AppStateRust {
    theme: i32,
    save_window_size: bool,
    check_updates: bool,
    mini_on_top: bool,
    accent: QString,
    compact_rows: bool,
    hidden_columns: QStringList,
    window_width: i32,
    window_height: i32,
    version: QString,
    replay_gain: i32,
    logs_folder: QString,
}

impl Default for AppStateRust {
    fn default() -> Self {
        let size = WindowSize::default();
        Self {
            theme: 0,
            save_window_size: false,
            check_updates: true,
            mini_on_top: true,
            accent: QString::from("amber"),
            compact_rows: false,
            hidden_columns: QStringList::default(),
            window_width: size.width as i32,
            window_height: size.height as i32,
            version: QString::from(env!("CARGO_PKG_VERSION")),
            replay_gain: 0,
            logs_folder: QString::default(),
        }
    }
}

impl cxx_qt::Initialize for qobject::AppState {
    fn initialize(mut self: Pin<&mut Self>) {
        let ui = settings::ui().get().clone();
        let size = if ui.save_window_size {
            ui.window_size
        } else {
            WindowSize::default()
        };
        self.as_mut().set_theme(i32::from(ui.theme));
        self.as_mut().set_save_window_size(ui.save_window_size);
        self.as_mut().set_check_updates(ui.check_updates);
        self.as_mut().set_mini_on_top(ui.mini_on_top);
        self.as_mut().set_accent(QString::from(&ui.accent));
        self.as_mut().set_compact_rows(ui.compact_rows);
        let mut hidden = QList::<QString>::default();
        for name in &ui.hidden_columns {
            hidden.append(QString::from(name));
        }
        self.as_mut().set_hidden_columns(QStringList::from(&hidden));
        self.as_mut().set_window_width(size.width as i32);
        self.as_mut().set_window_height(size.height as i32);
        self.as_mut()
            .set_replay_gain(match settings::replay_gain() {
                ReplayGainMode::Off => 0,
                ReplayGainMode::Track => 1,
                ReplayGainMode::Album => 2,
            });
        // The logs are written beside the settings.
        let logs = platform::internal_dir();
        self.as_mut().set_logs_folder(
            QUrl::from_local_file(&QString::from(&*logs.to_string_lossy())).to_qstring(),
        );
        self.as_mut()
            .on_replay_gain_changed(|app| {
                bus::emit(SetReplayGain(match *app.replay_gain() {
                    1 => ReplayGainMode::Track,
                    2 => ReplayGainMode::Album,
                    _ => ReplayGainMode::Off,
                }));
            })
            .release();
        self.as_mut()
            .on_theme_changed(|app| {
                let theme = Theme::from(*app.theme());
                settings::ui().update(|ui| ui.theme = theme);
            })
            .release();
        self.as_mut()
            .on_save_window_size_changed(|app| {
                let save = *app.save_window_size();
                settings::ui().update(|ui| ui.save_window_size = save);
            })
            .release();
        self.as_mut()
            .on_check_updates_changed(|app| {
                let check = *app.check_updates();
                settings::ui().update(|ui| ui.check_updates = check);
            })
            .release();
        self.as_mut()
            .on_mini_on_top_changed(|app| {
                let on_top = *app.mini_on_top();
                settings::ui().update(|ui| ui.mini_on_top = on_top);
            })
            .release();
        self.as_mut()
            .on_accent_changed(|app| {
                let accent = app.accent().to_string();
                settings::ui().update(|ui| ui.accent = accent);
            })
            .release();
        self.as_mut()
            .on_compact_rows_changed(|app| {
                let compact = *app.compact_rows();
                settings::ui().update(|ui| ui.compact_rows = compact);
            })
            .release();
        self.as_mut()
            .on_hidden_columns_changed(|app| {
                let hidden = QList::<QString>::from(app.hidden_columns())
                    .iter()
                    .map(|name| name.to_string())
                    .collect();
                settings::ui().update(|ui| ui.hidden_columns = hidden);
            })
            .release();
    }
}

impl qobject::AppState {
    fn window_closing(&self, width: i32, height: i32) {
        if self.save_window_size {
            settings::ui().update(|ui| {
                ui.window_size = WindowSize {
                    width: width.max(1) as usize,
                    height: height.max(1) as usize,
                };
            });
        }
    }

    fn set_visible(&self, visible: bool) {
        bus::emit(AppVisibilityChanged(visible));
    }
}
