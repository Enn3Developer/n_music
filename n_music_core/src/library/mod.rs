//! Scan results kept between launches: the library database and a shared cover store.

pub mod catalog;
pub mod covers;
pub mod db;
pub mod fingerprint;
pub mod query;
pub mod service;
pub mod track;
pub mod user_data;

use std::io;
use std::path::{Path, PathBuf};

/// Where the library keeps its files.
#[derive(Clone, Debug)]
pub struct LibraryPaths {
    /// The SQLite database. Kept with the app data, not the cache: it will also hold data the
    /// user creates (play counts, playlists), which must survive a cache cleanup.
    pub database: PathBuf,
    /// Covers can always be rebuilt from the files, so they live in the cache directory.
    pub covers: PathBuf,
}

impl LibraryPaths {
    pub fn new(internal_dir: &Path, cache_dir: &Path) -> Self {
        Self {
            database: internal_dir.join("library.db"),
            covers: cache_dir.join("covers"),
        }
    }

    pub fn open_db(&self) -> db::Result<db::LibraryDb> {
        db::LibraryDb::open(&self.database, &self.covers)
    }
}

/// Writes `bytes` to `path` atomically (temporary file in the same directory, then rename).
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(dir)?;
    file.write_all(bytes)?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}
