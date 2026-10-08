//! Where track bytes come from.
//!
//! A [`Locator`] identifies a track independently of how it is stored, and a [`StreamProvider`]
//! turns locators into readable streams. Everything that reads audio goes through a provider,
//! so new backends (Android documents, HTTP, Telegram, ...) only have to implement
//! [`StreamProvider`].

pub(crate) mod cache;
mod local;
mod telegram;
mod web;

pub use local::LocalProvider;
pub use web::WebProvider;

use cache::StreamCache;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
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
    /// An `http` or `https` address: an M3U or PLS playlist as a library root, a file as a
    /// track.
    Web(String),
    /// A chat on Telegram as a library root, `telegram:<chat id>`: a channel, a group or Saved
    /// Messages. The audio files sent in it are its tracks.
    TelegramChat(String),
    /// An audio file sent in a chat on Telegram, `telegram:<chat id>/<message id>`. Like a
    /// [`Locator::Document`], its file name is kept alongside: it gives the format and stands in
    /// for a missing title.
    TelegramAudio { uri: String, name: String },
}

impl Locator {
    /// A [`Locator::Web`] for `address`, written the way the scan writes the tracks of a
    /// playlist; `None` when it is not an `http` or `https` address.
    pub fn web(address: &str) -> Option<Self> {
        web::normalize(address).map(Locator::Web)
    }

    /// The [`Locator::TelegramChat`] of the chat `chat`, by its Bot API style id.
    pub fn telegram_chat(chat: i64) -> Self {
        Locator::TelegramChat(telegram::chat_uri(chat))
    }

    /// The [`Locator::TelegramAudio`] of the file sent as `message` in `chat`, named `name`.
    pub fn telegram_audio(chat: i64, message: i32, name: String) -> Self {
        Locator::TelegramAudio {
            uri: telegram::audio_uri(chat, message),
            name,
        }
    }

    /// A [`Locator::TelegramChat`] for `location` when it is the text of one, written the way
    /// [`Locator::telegram_chat`] writes it.
    pub fn telegram(location: &str) -> Option<Self> {
        match telegram::parse(location.trim()) {
            Some((chat, None)) => Some(Locator::telegram_chat(chat)),
            _ => None,
        }
    }

    /// The chat of a [`Locator::TelegramChat`] or [`Locator::TelegramAudio`], and the message
    /// the latter was sent as.
    pub fn telegram_ids(&self) -> Option<(i64, Option<i32>)> {
        match self {
            Locator::TelegramChat(uri) | Locator::TelegramAudio { uri, .. } => telegram::parse(uri),
            _ => None,
        }
    }

    /// Streamed from elsewhere rather than read on this device: the stream cache keeps copies
    /// of these.
    pub fn is_remote(&self) -> bool {
        matches!(
            self,
            Locator::Web(_) | Locator::TelegramChat(_) | Locator::TelegramAudio { .. }
        )
    }

    /// Lower-case file extension, used as a format hint and for tag readers.
    pub fn extension(&self) -> Option<String> {
        Path::new(&*self.file_name())
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase())
    }

    /// File name without its extension, shown when a track has no title tag.
    pub fn display_name(&self) -> String {
        let name = self.file_name();
        match (Path::new(&*name).file_stem(), self) {
            (Some(stem), _) => stem.to_string_lossy().into_owned(),
            // An address without a path, like `https://example.com/`.
            (None, Locator::Web(address)) => web::host(address),
            (None, _) => name.into_owned(),
        }
    }

    fn file_name(&self) -> Cow<'_, str> {
        match self {
            Locator::Local(path) | Locator::DocumentTree(path) | Locator::TelegramChat(path) => {
                Cow::Borrowed(path)
            }
            Locator::Document { name, .. } | Locator::TelegramAudio { name, .. } => {
                Cow::Borrowed(name)
            }
            Locator::Web(address) => Cow::Owned(web::file_name(address)),
        }
    }
}

impl Display for Locator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Locator::Local(text)
            | Locator::DocumentTree(text)
            | Locator::Web(text)
            | Locator::TelegramChat(text) => f.write_str(text),
            Locator::Document { uri, .. } | Locator::TelegramAudio { uri, .. } => f.write_str(uri),
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
/// backend, so requests are routed by the locator alone. Remote tracks also go through the
/// stream cache, once the engine adds it.
#[derive(Default)]
pub struct Providers {
    local: Option<Arc<dyn StreamProvider>>,
    documents: Option<Arc<dyn StreamProvider>>,
    web: Option<Arc<dyn StreamProvider>>,
    telegram: Option<Arc<dyn StreamProvider>>,
    cache: Option<Arc<StreamCache>>,
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

    /// Serves [`Locator::Web`].
    pub fn with_web(mut self, provider: impl StreamProvider + 'static) -> Self {
        self.web = Some(Arc::new(provider));
        self
    }

    /// Keeps copies of remote tracks in `cache`.
    pub(crate) fn with_cache(mut self, cache: Arc<StreamCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Opens `locator` to play it: from the stream cache's copy when there is one, else from
    /// its backend, recording it into the cache as it is read.
    pub(crate) fn play(&self, locator: &Locator) -> io::Result<OpenedStream> {
        let provider = self.route(locator)?;
        match &self.cache {
            Some(cache) if locator.is_remote() => cache.play(locator, provider),
            _ => provider.open(locator),
        }
    }

    /// The stream cache has a copy of `locator`: it plays offline.
    pub(crate) fn has_copy(&self, locator: &Locator) -> bool {
        self.cache
            .as_ref()
            .is_some_and(|cache| cache.has_copy(locator))
    }

    fn route(&self, locator: &Locator) -> io::Result<&Arc<dyn StreamProvider>> {
        let provider = match locator {
            Locator::Local(_) => &self.local,
            Locator::DocumentTree(_) | Locator::Document { .. } => &self.documents,
            Locator::Web(_) => &self.web,
            Locator::TelegramChat(_) | Locator::TelegramAudio { .. } => &self.telegram,
        };
        provider.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                format!("No provider for {locator} on this platform"),
            )
        })
    }
}

impl StreamProvider for Providers {
    /// Reads from the backend, for tags to be up to date; a remote track it cannot reach is
    /// read from its copy.
    fn open(&self, locator: &Locator) -> io::Result<OpenedStream> {
        let opened = self.route(locator)?.open(locator);
        match &self.cache {
            Some(cache) if locator.is_remote() => cache.check(locator, opened),
            _ => opened,
        }
    }

    fn list_tracks(&self, root: &Locator) -> io::Result<Vec<TrackEntry>> {
        self.route(root)?.list_tracks(root)
    }
}

/// A backend that can enumerate and open tracks.
pub trait StreamProvider: Send + Sync {
    /// Opens `locator` for reading from the start.
    fn open(&self, locator: &Locator) -> io::Result<OpenedStream>;

    /// Lists the audio tracks in `root`.
    fn list_tracks(&self, root: &Locator) -> io::Result<Vec<TrackEntry>>;
}

/// Stable version stamp from a size and a modification time in milliseconds.
pub fn version_stamp(size: u64, modified_ms: u64) -> u64 {
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(&size.to_le_bytes());
    bytes[8..].copy_from_slice(&modified_ms.to_le_bytes());
    xxhash_rust::xxh3::xxh3_64(&bytes)
}
