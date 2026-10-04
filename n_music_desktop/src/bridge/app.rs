//! The window's preferences and what it reports to the core.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
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
        #[qproperty(i32, window_width)]
        #[qproperty(i32, window_height)]
        #[qproperty(QString, version)]
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
use core::pin::Pin;
use cxx_qt_lib::QString;
use n_music_core::messages::AppVisibilityChanged;

pub struct AppStateRust {
    theme: i32,
    save_window_size: bool,
    window_width: i32,
    window_height: i32,
    version: QString,
}

impl Default for AppStateRust {
    fn default() -> Self {
        let size = WindowSize::default();
        Self {
            theme: 0,
            save_window_size: false,
            window_width: size.width as i32,
            window_height: size.height as i32,
            version: QString::from(env!("CARGO_PKG_VERSION")),
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
        self.as_mut().set_window_width(size.width as i32);
        self.as_mut().set_window_height(size.height as i32);
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
        crate::bus::emit(AppVisibilityChanged(visible));
    }
}
