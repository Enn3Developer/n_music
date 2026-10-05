//! The window icon: see `cpp/icon.h`.

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("n_music_desktop/cpp/icon.h");
        /// Gives every window the app's icon.
        #[cxx_name = "installWindowIcon"]
        fn install_window_icon();
    }
}

pub use ffi::install_window_icon as install;
