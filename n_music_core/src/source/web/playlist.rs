//! What an M3U or PLS playlist lists.

use std::io;
use url::Url;

/// The text of a playlist: UTF-8, as M3U8 and most M3U are, or else Latin-1.
pub(super) fn decode(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes)
        .unwrap_or_else(|error| error.into_bytes().into_iter().map(char::from).collect())
}

/// The files `text` lists, resolved against `base`, in order. Entries off the web, like the
/// paths of a playlist made for a computer, are left out.
pub(super) fn parse(text: &str, base: &Url) -> io::Result<Vec<Url>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let invalid = |message: &str| io::Error::new(io::ErrorKind::InvalidData, message.to_string());
    // A web page, an XML playlist or an audio file.
    if text.trim_start().starts_with('<') || text.contains('\0') {
        return Err(invalid("is not an M3U or PLS playlist"));
    }
    let entries = match pls(text) {
        entries if !entries.is_empty() || is_pls(text) => entries,
        _ => m3u(text).ok_or_else(|| invalid("is an HLS stream, not a playlist of files"))?,
    };
    Ok(entries
        .into_iter()
        .filter_map(|entry| {
            let mut file = base.join(entry).ok()?;
            file.set_fragment(None);
            matches!(file.scheme(), "http" | "https").then_some(file)
        })
        .collect())
}

/// A PLS file starts with `[playlist]`.
fn is_pls(text: &str) -> bool {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .is_some_and(|line| line.eq_ignore_ascii_case("[playlist]"))
}

/// The `FileN=` entries of a PLS file, by `N`.
fn pls(text: &str) -> Vec<&str> {
    let mut entries: Vec<(u64, &str)> = text
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let key = key.trim().to_ascii_lowercase();
            let number = key.strip_prefix("file")?.parse().ok()?;
            Some((number, value.trim()))
        })
        .collect();
    entries.sort_by_key(|&(number, _)| number);
    entries.into_iter().map(|(_, entry)| entry).collect()
}

/// The entries of an M3U list; `None` for the segments of an HLS stream.
fn m3u(text: &str) -> Option<Vec<&str>> {
    let mut entries = vec![];
    for line in text.lines().map(str::trim) {
        if line.starts_with("#EXT-X-") {
            return None;
        }
        if !line.is_empty() && !line.starts_with('#') {
            entries.push(line);
        }
    }
    Some(entries)
}
