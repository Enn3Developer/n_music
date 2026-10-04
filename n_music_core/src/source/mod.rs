//! Where track bytes come from.
//!
//! A [`Locator`] identifies a track independently of how it is stored, and a [`StreamProvider`]
//! turns locators into readable streams. Everything that reads audio goes through a provider,
//! so new backends (Android documents, HTTP, ...) only have to implement [`StreamProvider`].

mod local;

pub use local::LocalProvider;

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::Path;
use std::sync::Arc;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSource;

/// Identity of a track (or of a library root) across the whole app.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Locator {
    /// A path on the local file system.
    Local(String),
    /// A folder picked through Android's Storage Access Framework (a `content://` tree URI).
    DocumentTree(String),
    /// A file inside a [`Locator::DocumentTree`]. Document URIs do not always contain the file
    /// name (e.g. `msf:1234`), so the display name is kept alongside.
    Document { uri: String, name: String },
}

impl Locator {
    /// Lower-case file extension, used as a format hint and for tag readers.
    pub fn extension(&self) -> Option<String> {
        Path::new(self.file_name())
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase())
    }

    /// File name without its extension, shown when a track has no title tag.
    pub fn display_name(&self) -> String {
        let name = self.file_name();
        Path::new(name)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| name.to_string())
    }

    fn file_name(&self) -> &str {
        match self {
            Locator::Local(path) | Locator::DocumentTree(path) => path,
            Locator::Document { name, .. } => name,
        }
    }
}

impl Display for Locator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Locator::Local(path) | Locator::DocumentTree(path) => f.write_str(path),
            Locator::Document { uri, .. } => f.write_str(uri),
        }
    }
}

/// A track found while listing a library root.
#[derive(Clone, Debug)]
pub struct TrackEntry {
    pub locator: Locator,
    /// Changes whenever the track's content may have changed (e.g. size + modification time).
    /// `None` when the backend cannot tell, so cached metadata is never trusted.
    pub version: Option<u64>,
}

/// A freshly opened track, positioned at its first byte.
pub struct OpenedStream {
    pub source: Box<dyn MediaSource>,
    pub extension: Option<String>,
}

impl OpenedStream {
    /// Probe hint for Symphonia; content sniffing still works without an extension.
    pub fn hint(&self) -> Hint {
        let mut hint = Hint::new();
        if let Some(extension) = &self.extension {
            hint.with_extension(extension);
        }
        hint
    }
}

/// The set of backends available on this platform. Each [`Locator`] kind is served by one
/// backend, so requests are routed by the locator alone.
#[derive(Default)]
pub struct Providers {
    local: Option<Arc<dyn StreamProvider>>,
    documents: Option<Arc<dyn StreamProvider>>,
}

impl Providers {
    /// Serves [`Locator::Local`].
    pub fn with_local(mut self, provider: impl StreamProvider + 'static) -> Self {
        self.local = Some(Arc::new(provider));
        self
    }

    /// Serves [`Locator::DocumentTree`] and [`Locator::Document`].
    pub fn with_documents(mut self, provider: impl StreamProvider + 'static) -> Self {
        self.documents = Some(Arc::new(provider));
        self
    }

    fn route(&self, locator: &Locator) -> io::Result<&dyn StreamProvider> {
        let provider = match locator {
            Locator::Local(_) => &self.local,
            Locator::DocumentTree(_) | Locator::Document { .. } => &self.documents,
        };
        provider.as_deref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                format!("No provider for {locator} on this platform"),
            )
        })
    }
}

impl StreamProvider for Providers {
    fn open(&self, locator: &Locator) -> io::Result<OpenedStream> {
        self.route(locator)?.open(locator)
    }

    fn list_tracks(&self, root: &Locator) -> io::Result<Vec<TrackEntry>> {
        self.route(root)?.list_tracks(root)
    }
}

/// A backend that can enumerate and open tracks.
pub trait StreamProvider: Send + Sync {
    /// Opens `locator` for reading from the start.
    fn open(&self, locator: &Locator) -> io::Result<OpenedStream>;

    /// Lists the audio tracks directly inside `root`.
    fn list_tracks(&self, root: &Locator) -> io::Result<Vec<TrackEntry>>;
}

/// Stable version stamp from a size and a modification time in milliseconds.
pub fn version_stamp(size: u64, modified_ms: u64) -> u64 {
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(&size.to_le_bytes());
    bytes[8..].copy_from_slice(&modified_ms.to_le_bytes());
    xxhash_rust::xxh3::xxh3_64(&bytes)
}
