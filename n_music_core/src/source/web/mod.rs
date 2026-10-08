//! Playlists and files on the web, over HTTP or HTTPS: an M3U or PLS playlist is a library, the
//! files it lists its tracks.

mod file;
mod playlist;

use super::{Locator, OpenedStream, StreamProvider, TrackEntry};
use percent_encoding::percent_decode_str;
use std::io;
use std::time::Duration;
use ureq::{Agent, ResponseExt};
use url::Url;

/// A file is taken to stay the same at its address: only reloading the metadata reads it again.
const VERSION: u64 = 0;
/// A playlist this large is something else.
const PLAYLIST_LIMIT: u64 = 16 * 1024 * 1024;
const PLAYLIST_TIMEOUT: Duration = Duration::from_secs(30);

/// Reads M3U and PLS playlists on the web, and streams the files they list.
pub struct WebProvider {
    agent: Agent,
}

impl Default for WebProvider {
    fn default() -> Self {
        let config = Agent::config_builder()
            .user_agent(concat!("N Music/", env!("CARGO_PKG_VERSION")))
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_recv_response(Some(Duration::from_secs(20)))
            .build();
        Self {
            agent: config.into(),
        }
    }
}

impl StreamProvider for WebProvider {
    fn open(&self, locator: &Locator) -> io::Result<OpenedStream> {
        let Locator::Web(address) = locator else {
            return Err(unsupported(locator));
        };
        Ok(OpenedStream {
            source: Box::new(file::open(self.agent.clone(), address)?),
            extension: locator.extension(),
        })
    }

    fn list_tracks(&self, root: &Locator) -> io::Result<Vec<TrackEntry>> {
        let Locator::Web(address) = root else {
            return Err(unsupported(root));
        };
        let mut response = self
            .agent
            .get(address)
            .config()
            .timeout_recv_body(Some(PLAYLIST_TIMEOUT))
            .build()
            .call()
            .map_err(|error| failed(address, error))?;
        // Its entries are relative to where it was found, after redirects.
        let base = Url::parse(&response.get_uri().to_string()).map_err(|error| {
            io::Error::new(io::ErrorKind::InvalidData, format!("{address}: {error}"))
        })?;
        let bytes = response
            .body_mut()
            .with_config()
            .limit(PLAYLIST_LIMIT)
            .read_to_vec()
            .map_err(|error| failed(address, error))?;
        let files = playlist::parse(&playlist::decode(bytes), &base)
            .map_err(|error| io::Error::new(error.kind(), format!("{address} {error}")))?;
        Ok(files
            .into_iter()
            .map(|file| TrackEntry {
                locator: Locator::Web(file.into()),
                version: Some(VERSION),
            })
            .collect())
    }
}

/// `address` written the way playlists' entries are, when it is an `http` or `https` one.
pub(super) fn normalize(address: &str) -> Option<String> {
    let url = Url::parse(address.trim()).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some()).then(|| url.into())
}

/// The decoded last part of the path of `address`; empty when it has none.
pub(super) fn file_name(address: &str) -> String {
    let Ok(url) = Url::parse(address) else {
        return String::new();
    };
    url.path_segments()
        .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
        .map(|segment| percent_decode_str(segment).decode_utf8_lossy().into_owned())
        .unwrap_or_default()
}

/// The host of `address`, or all of it when it has none.
pub(super) fn host(address: &str) -> String {
    Url::parse(address)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_else(|| address.to_string())
}

/// `error` from fetching `address`, as an IO error of the closest kind.
fn failed(address: &str, error: ureq::Error) -> io::Error {
    let kind = match &error {
        ureq::Error::StatusCode(404 | 410) => io::ErrorKind::NotFound,
        ureq::Error::StatusCode(401 | 403) => io::ErrorKind::PermissionDenied,
        ureq::Error::StatusCode(400..500) => io::ErrorKind::InvalidInput,
        ureq::Error::Timeout(_) => io::ErrorKind::TimedOut,
        ureq::Error::Io(error) => error.kind(),
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, format!("Could not fetch {address}: {error}"))
}

fn unsupported(locator: &Locator) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!("{locator} is not a web address"),
    )
}
