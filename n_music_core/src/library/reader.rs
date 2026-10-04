//! Reads what the library keeps about a track from the file: its properties, tags and
//! embedded cover.

use crate::audio::{self, CODEC_REGISTRY};
use crate::library::fingerprint::fingerprint;
use crate::library::track::TrackInfo;
use crate::source::{Locator, StreamProvider};
use multitag::data::Picture;
use multitag::Tag;
use std::io;
use symphonia::core::formats::TrackType;
use symphonia_core::meta::{MetadataContainer, StandardTag, StandardVisualKey};

/// Everything the library keeps about a track, plus the embedded cover's encoded bytes.
/// Symphonia reads tags and pictures in the same pass; the file is only opened a second time
/// (by multitag) when that pass misses the title, the artist or the cover.
pub fn read_info(
    provider: &dyn StreamProvider,
    locator: &Locator,
) -> io::Result<(TrackInfo, Option<Vec<u8>>)> {
    let mut format = audio::open(provider, locator)?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| io::Error::other("No audio track"))?;
    let track_id = track.id;
    let length = audio::length(track).ok_or_else(|| io::Error::other("No audio duration"))?;
    let mut info = TrackInfo::placeholder(locator.clone());
    info.title.clear();
    info.length = length;
    if let Some(params) = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
    {
        info.codec = CODEC_REGISTRY
            .get_audio_decoder(params.codec)
            .map(|decoder| decoder.codec.info.short_name.to_string());
        info.sample_rate = params.sample_rate;
        info.channels = params
            .channels
            .as_ref()
            .map(|channels| channels.count() as u32);
        info.bits_per_sample = params.bits_per_sample;
    }

    let mut tags = TagReader::default();
    let mut cover: Option<(bool, Vec<u8>)> = None;
    audio::for_each_container(format.as_mut(), track_id, |container| {
        tags.read_container(&mut info, container);
        for visual in &container.visuals {
            let front = visual.usage == Some(StandardVisualKey::FrontCover);
            if cover
                .as_ref()
                .is_none_or(|(was_front, _)| front && !was_front)
            {
                cover = Some((front, visual.data.to_vec()));
            }
        }
    });
    info.fingerprint = fingerprint(format.as_mut(), track_id);
    // Close the file before multitag may open it again.
    drop(format);
    info.year = tags.year;
    let mut cover = cover.map(|(_, data)| data);

    if info.title.is_empty() || info.artists.is_empty() || cover.is_none() {
        if let Some(tag) = read_tag(provider, locator) {
            if info.title.is_empty() {
                if let Some(title) = tag.title() {
                    info.title = title.to_string();
                }
            }
            if info.artists.is_empty() {
                for artist in tag.artists().unwrap_or_default() {
                    if !artist.is_empty() && !info.artists.contains(&artist) {
                        info.artists.push(artist);
                    }
                }
            }
            if cover.is_none() {
                cover = tag_cover(tag);
            }
        }
    }
    if info.title.is_empty() {
        info.title = locator.display_name();
    }
    Ok((info, cover.filter(|data| !data.is_empty())))
}

/// Reads the tags with multitag, for what Symphonia's metadata does not expose.
fn read_tag(provider: &dyn StreamProvider, locator: &Locator) -> Option<Tag> {
    let stream = provider.open(locator).ok()?;
    let extension = stream.extension?;
    Tag::read_from(&extension, stream.source).ok()
}

/// The embedded cover of a tag read by multitag, for files Symphonia finds no picture in.
pub fn tag_cover(tag: Tag) -> Option<Vec<u8>> {
    if let Some(cover) = tag.get_album_info().and_then(|album| album.cover) {
        return Some(cover.data);
    }
    let cover = match tag {
        Tag::OpusTag { inner } => inner.pictures().first().cloned().map(Picture::from),
        Tag::Id3Tag { inner } => inner.pictures().next().cloned().map(Picture::from),
        _ => None,
    };
    cover.map(|cover| cover.data)
}

/// Folds Symphonia's standard tags into a [`TrackInfo`]. The first value of a single-valued
/// field wins; multi-valued fields keep every distinct value in order, from the first
/// container that has any.
#[derive(Default)]
struct TagReader {
    year: Option<i32>,
    /// Lower is better: a release date beats a recording date, which beats an original one.
    year_rank: u8,
}

impl TagReader {
    fn read_container(&mut self, info: &mut TrackInfo, container: &MetadataContainer) {
        // An earlier container's lists win whole; merging would mix up two tags' artists.
        let artists = std::mem::take(&mut info.artists);
        let genres = std::mem::take(&mut info.genres);
        for tag in &container.tags {
            if let Some(tag) = &tag.std {
                self.read(info, tag);
            }
        }
        if !artists.is_empty() {
            info.artists = artists;
        }
        if !genres.is_empty() {
            info.genres = genres;
        }
    }

    fn read(&mut self, info: &mut TrackInfo, tag: &StandardTag) {
        fn set<T>(field: &mut Option<T>, value: T) {
            field.get_or_insert(value);
        }
        fn push(values: &mut Vec<String>, value: &str) {
            let value = value.trim();
            if !value.is_empty() && !values.iter().any(|known| known == value) {
                values.push(value.to_string());
            }
        }
        fn text(value: &str) -> Option<String> {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_string())
        }
        let number = |value: u64| u32::try_from(value).ok().filter(|value| *value > 0);
        match tag {
            StandardTag::TrackTitle(value) if info.title.is_empty() => {
                info.title = value.trim().to_string()
            }
            StandardTag::Artist(value) => push(&mut info.artists, value),
            StandardTag::Genre(value) => push(&mut info.genres, value),
            StandardTag::Album(value) => {
                if let Some(value) = text(value) {
                    set(&mut info.album, value)
                }
            }
            StandardTag::AlbumArtist(value) => {
                if let Some(value) = text(value) {
                    set(&mut info.album_artist, value)
                }
            }
            StandardTag::TrackNumber(value) => {
                if let Some(value) = number(*value) {
                    set(&mut info.track_number, value)
                }
            }
            StandardTag::TrackTotal(value) => {
                if let Some(value) = number(*value) {
                    set(&mut info.track_total, value)
                }
            }
            StandardTag::DiscNumber(value) => {
                if let Some(value) = number(*value) {
                    set(&mut info.disc_number, value)
                }
            }
            StandardTag::DiscTotal(value) => {
                if let Some(value) = number(*value) {
                    set(&mut info.disc_total, value)
                }
            }
            StandardTag::ReleaseYear(year) => self.year(1, Some(i32::from(*year))),
            StandardTag::ReleaseDate(date) => self.year(1, parse_year(date)),
            StandardTag::RecordingYear(year) => self.year(2, Some(i32::from(*year))),
            StandardTag::RecordingDate(date) => self.year(2, parse_year(date)),
            StandardTag::OriginalReleaseDate(date) => self.year(3, parse_year(date)),
            tag if info.replay_gain.read(tag) => {}
            _ => {}
        }
    }

    fn year(&mut self, rank: u8, year: Option<i32>) {
        if let Some(year) = year.filter(|year| *year > 0) {
            if self.year.is_none() || rank < self.year_rank {
                self.year = Some(year);
                self.year_rank = rank;
            }
        }
    }
}

/// The year of a date like `2021`, `2021-03-04` or `2021-03-04T10:00:00`.
fn parse_year(date: &str) -> Option<i32> {
    let date = date.trim();
    let digits = date.len() - date.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    (digits == 4).then(|| date[..4].parse().ok()).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Locator;
    use std::sync::Arc;

    #[test]
    fn folds_standard_tags() {
        let mut info = TrackInfo::placeholder(Locator::Local("a.opus".into()));
        info.title.clear();
        let mut reader = TagReader::default();
        let text = |value: &str| Arc::new(value.to_string());
        for tag in [
            StandardTag::TrackTitle(text("Song")),
            StandardTag::TrackTitle(text("Other")),
            StandardTag::Artist(text("A")),
            StandardTag::Artist(text("B")),
            StandardTag::Artist(text("A")),
            StandardTag::RecordingDate(text("1999-01-02")),
            StandardTag::ReleaseDate(text("2001")),
            StandardTag::TrackNumber(3),
            StandardTag::ReplayGainTrackGain(text("-6.50 dB")),
        ] {
            reader.read(&mut info, &tag);
        }
        assert_eq!(info.title, "Song");
        assert_eq!(info.artists, ["A", "B"]);
        assert_eq!(reader.year, Some(2001));
        assert_eq!(info.track_number, Some(3));
        assert_eq!(info.replay_gain.track_gain, Some(-6.5));
    }

    #[test]
    fn parses_years() {
        assert_eq!(parse_year("2021-03-04T10:00"), Some(2021));
        assert_eq!(parse_year("1999"), Some(1999));
        assert_eq!(parse_year("99"), None);
        assert_eq!(parse_year("March 2020"), None);
    }
}
