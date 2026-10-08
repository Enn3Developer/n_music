//! Lists the audio files sent in Telegram chats and opens them, read while a thread downloads
//! them ahead, a chunk at a time.

use super::account::{TelegramAccount, TIMEOUT};
use super::chunks::{Chunks, Fetch};
use super::TelegramError;
use crate::source::ahead::{Part, Remote, RemoteFile, MIN_AHEAD};
use crate::source::{
    version_stamp, Locator, OpenedStream, StreamProvider, TrackEntry, AUDIO_EXTENSIONS,
};
use grammers_client::media::{Document, Downloadable, Media};
use grammers_client::message::Message;
use grammers_client::session::types::PeerRef;
use grammers_client::{tl, Client, InvocationError};
use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A chunk that takes longer is given up, and asked for again.
const CHUNK_TIMEOUT: Duration = Duration::from_secs(60);

/// Serves the chats of a [`TelegramAccount`].
pub(crate) struct TelegramProvider {
    account: Arc<TelegramAccount>,
}

impl TelegramProvider {
    pub(crate) fn new(account: Arc<TelegramAccount>) -> Self {
        Self { account }
    }
}

impl StreamProvider for TelegramProvider {
    fn open(&self, locator: &Locator) -> io::Result<OpenedStream> {
        let (Locator::TelegramAudio { .. }, Some((chat, Some(message)))) =
            (locator, locator.telegram_ids())
        else {
            return Err(unsupported(locator));
        };
        let peer = self.account.peer(chat)?;
        let document = document(&self.account, peer, message)?;
        let len = document.size().map_or(0, |size| size as u64);
        let document = Arc::new(Mutex::new(document));
        let stale = Arc::new(AtomicBool::new(false));
        let remote = TelegramRemote {
            account: self.account.clone(),
            peer,
            message,
            name: locator.to_string(),
            chunks: Chunks::new(len, chunks(&self.account, &document, &stale)),
            document,
            stale,
        };
        let first = remote.part(0, Some(len.min(MIN_AHEAD)))?;
        let file = RemoteFile::open(remote, first, true, Some(len), "telegram download")?;
        Ok(OpenedStream {
            source: Box::new(file),
            extension: locator.extension(),
        })
    }

    /// The audio files sent in the chat, as music or as files, in the order they were sent.
    fn list_tracks(&self, root: &Locator) -> io::Result<Vec<TrackEntry>> {
        let (Locator::TelegramChat(_), Some((chat, None))) = (root, root.telegram_ids()) else {
            return Err(unsupported(root));
        };
        let peer = self.account.peer(chat)?;
        let client = self.account.client();
        let mut tracks = BTreeMap::new();
        for filter in [
            tl::enums::MessagesFilter::InputMessagesFilterMusic,
            tl::enums::MessagesFilter::InputMessagesFilterDocument,
        ] {
            let mut messages = client.search_messages(peer).filter(filter);
            // A page at a time: a chat with many files takes long to list.
            while let Some(message) = self
                .account
                .block_on(TIMEOUT, async { Ok(messages.next().await?) })?
            {
                if let Some(track) = audio(chat, &message) {
                    tracks.entry(message.id()).or_insert(track);
                }
            }
        }
        Ok(tracks.into_values().collect())
    }
}

/// The audio file sent as `message` in `chat`; `None` when it sent none, or a voice message.
fn audio(chat: i64, message: &Message) -> Option<TrackEntry> {
    let Some(Media::Document(document)) = message.media() else {
        return None;
    };
    let Some(tl::enums::Document::Document(file)) = &document.raw.document else {
        return None;
    };
    let mut audio = None;
    let mut file_name = None;
    for attribute in &file.attributes {
        match attribute {
            tl::enums::DocumentAttribute::Audio(attribute) => audio = Some(attribute),
            tl::enums::DocumentAttribute::Filename(attribute) => {
                file_name = Some(attribute.file_name.as_str())
            }
            // A video, maybe with sound: not a track.
            tl::enums::DocumentAttribute::Video(_) => return None,
            _ => {}
        }
    }
    if audio.is_some_and(|audio| audio.voice) {
        return None;
    }
    let extension = file_name
        .and_then(|name| Path::new(name).extension())
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .filter(|extension| AUDIO_EXTENSIONS.contains(&extension.as_str()))
        .or_else(|| mime_extension(&file.mime_type).map(str::to_string));
    if audio.is_none() && extension.is_none() && !file.mime_type.starts_with("audio/") {
        return None;
    }
    let title = audio.and_then(|audio| audio.title.as_deref());
    let performer = audio.and_then(|audio| audio.performer.as_deref());
    let name = track_name(
        file_name,
        title,
        performer,
        extension.as_deref(),
        message.id(),
    );
    Some(TrackEntry {
        locator: Locator::telegram_audio(chat, message.id(), name),
        // A message edited to send another file has another document.
        version: Some(version_stamp(file.size as u64, file.id as u64)),
    })
}

/// The file name of an audio file: the one it was sent with, else its performer and title,
/// else its message; with an extension that tells its format when one is known.
fn track_name(
    file_name: Option<&str>,
    title: Option<&str>,
    performer: Option<&str>,
    extension: Option<&str>,
    message: i32,
) -> String {
    // Kept on one level: a slash would cut the name short.
    let clean = |text: &str| text.trim().replace(['/', '\\'], "∕");
    let nonblank = |text: &&str| !text.trim().is_empty();
    if let Some(name) = file_name.filter(nonblank) {
        let name = clean(name);
        let known = extension.is_some_and(|extension| {
            Path::new(&name)
                .extension()
                .is_some_and(|own| own.eq_ignore_ascii_case(extension))
        });
        return match extension.filter(|_| !known) {
            Some(extension) => format!("{name}.{extension}"),
            None => name,
        };
    }
    let stem = match (performer.filter(nonblank), title.filter(nonblank)) {
        (Some(performer), Some(title)) => format!("{} - {}", clean(performer), clean(title)),
        (None, Some(title)) => clean(title),
        _ => format!("Telegram {message}"),
    };
    match extension {
        Some(extension) => format!("{stem}.{extension}"),
        None => stem,
    }
}

/// The extension of the audio format named by `mime`, when it is one.
fn mime_extension(mime: &str) -> Option<&'static str> {
    let mime = mime.split(';').next()?.trim().to_ascii_lowercase();
    Some(match mime.as_str() {
        "audio/mpeg" | "audio/mp3" | "audio/mpeg3" | "audio/x-mpeg" => "mp3",
        "audio/flac" | "audio/x-flac" => "flac",
        "audio/ogg" | "application/ogg" | "audio/vorbis" => "ogg",
        "audio/opus" => "opus",
        "audio/mp4" | "audio/m4a" | "audio/x-m4a" | "audio/aac-adts" => "m4a",
        "audio/aac" | "audio/x-aac" => "aac",
        "audio/wav" | "audio/x-wav" | "audio/wave" | "audio/vnd.wave" => "wav",
        "audio/aiff" | "audio/x-aiff" => "aiff",
        "audio/webm" => "webm",
        "audio/x-matroska" => "mka",
        _ => return None,
    })
}

/// The file sent as `message`, with a file reference Telegram takes now.
fn document(
    account: &TelegramAccount,
    peer: PeerRef,
    message: i32,
) -> Result<Document, TelegramError> {
    let client = account.client();
    let messages = account.block_on(TIMEOUT, async {
        Ok(client.get_messages_by_id(peer, &[message]).await?)
    })?;
    match messages
        .into_iter()
        .next()
        .flatten()
        .and_then(|message| message.media())
    {
        Some(Media::Document(document))
            if matches!(
                document.raw.document,
                Some(tl::enums::Document::Document(_))
            ) =>
        {
            Ok(document)
        }
        // Deleted, or edited to send something else.
        _ => Err(TelegramError::NotFound),
    }
}

/// A file sent in a chat, downloaded a part at a time.
struct TelegramRemote {
    account: Arc<TelegramAccount>,
    peer: PeerRef,
    message: i32,
    name: String,
    document: Arc<Mutex<Document>>,
    /// Telegram did not take the file reference any more: the next part fetches the message
    /// again for a new one.
    stale: Arc<AtomicBool>,
    chunks: Arc<Chunks>,
}

impl Remote for TelegramRemote {
    fn part(&self, from: u64, to: Option<u64>) -> io::Result<Part> {
        if self.stale.swap(false, Ordering::Relaxed) {
            let fresh = document(&self.account, self.peer, self.message)?;
            *self.document.lock().unwrap() = fresh;
        }
        Ok(self.chunks.part(from, to))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Downloads the chunks of `document`; marks it `stale` when Telegram asks for a new file
/// reference.
fn chunks(
    account: &Arc<TelegramAccount>,
    document: &Arc<Mutex<Document>>,
    stale: &Arc<AtomicBool>,
) -> Fetch {
    let (account, document, stale) = (account.clone(), document.clone(), stale.clone());
    Box::new(move |offset, limit| {
        let client = account.client();
        let document = document.lock().unwrap().clone();
        let fetched = account.block_on(CHUNK_TIMEOUT, async {
            Ok(fetch(&client, &document, offset, limit).await)
        })?;
        fetched.map_err(|error| {
            if error.is("FILE_REFERENCE_*") {
                stale.store(true, Ordering::Relaxed);
            }
            TelegramError::from(error).into()
        })
    })
}

/// Downloads `limit` bytes of `document` from `offset`, from the data center that keeps it.
async fn fetch(
    client: &Client,
    document: &Document,
    offset: u64,
    limit: u64,
) -> Result<Vec<u8>, InvocationError> {
    let missing = || InvocationError::Io(io::Error::other("The file cannot be downloaded"));
    let location = document.to_raw_input_location().ok_or_else(missing)?;
    let Some(tl::enums::Document::Document(file)) = &document.raw.document else {
        return Err(missing());
    };
    let request = tl::functions::upload::GetFile {
        precise: false,
        cdn_supported: false,
        location,
        offset: offset as i64,
        limit: limit as i32,
    };
    let mut dc = file.dc_id;
    for _ in 0..2 {
        match client.invoke_in_dc(dc, &request).await {
            Ok(tl::enums::upload::File::File(file)) => return Ok(file.bytes),
            Ok(tl::enums::upload::File::CdnRedirect(_)) => {
                return Err(InvocationError::Io(io::Error::other(
                    "Telegram sent the file to a CDN",
                )))
            }
            Err(InvocationError::Rpc(error)) if error.name == "FILE_MIGRATE" => {
                dc = error.value.map_or(dc, |value| value as i32);
            }
            // Not signed in to that data center yet: grammers' own download signs in there.
            Err(error) if error.is("AUTH_KEY_UNREGISTERED") => break,
            Err(error) => return Err(error),
        }
    }
    let mut download = client
        .iter_download(document)
        .chunk_size(limit as i32)
        .skip_chunks((offset / limit) as i32);
    Ok(download.next().await?.unwrap_or_default())
}

fn unsupported(locator: &Locator) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!("{locator} is not a Telegram chat or audio file"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_keep_the_file_name() {
        assert_eq!(
            track_name(Some("Song.flac"), Some("Title"), None, Some("flac"), 5),
            "Song.flac"
        );
        assert_eq!(
            track_name(Some("Song.FLAC"), None, None, Some("flac"), 5),
            "Song.FLAC"
        );
    }

    #[test]
    fn names_get_the_extension_of_their_format() {
        assert_eq!(
            track_name(Some("Song"), None, None, Some("mp3"), 5),
            "Song.mp3"
        );
        assert_eq!(
            track_name(
                Some("AC/DC - Thunderstruck.mp3"),
                None,
                None,
                Some("mp3"),
                5
            ),
            "AC∕DC - Thunderstruck.mp3"
        );
    }

    #[test]
    fn names_without_a_file_name_come_from_the_tags_or_the_message() {
        assert_eq!(
            track_name(None, Some("Title"), Some("Artist"), Some("ogg"), 5),
            "Artist - Title.ogg"
        );
        assert_eq!(track_name(Some(" "), Some("Title"), None, None, 5), "Title");
        assert_eq!(
            track_name(None, None, Some("Artist"), Some("m4a"), 7),
            "Telegram 7.m4a"
        );
    }

    #[test]
    fn audio_formats_have_extensions() {
        assert_eq!(mime_extension("audio/mpeg"), Some("mp3"));
        assert_eq!(mime_extension("audio/x-flac; charset=binary"), Some("flac"));
        assert_eq!(mime_extension("application/ogg"), Some("ogg"));
        assert_eq!(mime_extension("video/mp4"), None);
        for extension in [
            "mp3", "flac", "ogg", "opus", "m4a", "aac", "wav", "aiff", "webm", "mka",
        ] {
            assert!(AUDIO_EXTENSIONS.contains(&extension), "{extension}");
        }
    }
}
