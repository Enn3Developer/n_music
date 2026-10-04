//! Where the desktop app keeps its files.

use std::path::PathBuf;

fn base_dirs() -> directories::BaseDirs {
    directories::BaseDirs::new().expect("No desktop application data directory is available")
}

fn ensure(dir: PathBuf) -> PathBuf {
    if let Err(error) = std::fs::create_dir_all(&dir) {
        log::error!("Could not create directory {}: {error}", dir.display());
    }
    dir
}

/// Where the app keeps its data: settings, the library database and logs.
pub fn internal_dir() -> PathBuf {
    ensure(base_dirs().data_local_dir().join("n_music"))
}

/// Where the app keeps what can be rebuilt (covers).
pub fn cache_dir() -> PathBuf {
    ensure(base_dirs().cache_dir().join("n_music"))
}
