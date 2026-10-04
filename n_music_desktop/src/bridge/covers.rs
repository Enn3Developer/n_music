//! Cover images for QML, with rounded corners: see `cpp/covers.h`.

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qqmlengine.h");
        type QQmlEngine = cxx_qt_lib::QQmlEngine;

        include!("n_music_desktop/cpp/covers.h");
        /// Serves `image://cover/` on `engine`.
        #[cxx_name = "installCoverProvider"]
        fn install_cover_provider(engine: Pin<&mut QQmlEngine>);
    }
}

pub use ffi::install_cover_provider as install;
