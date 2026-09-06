use crate::{remove_ext, Metadata, TrackTime, PROBE};
use multitag::Tag;
use std::ffi::OsStr;
use std::path::Path;
use std::{fs, io};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia_core::meta::StandardTag;

/// The basics where everything is built upon
pub struct MusicTrack {
    path: String,
    ext: String,
}

impl MusicTrack {
    pub fn new<P: AsRef<Path> + AsRef<OsStr> + Clone + Into<String>>(path: P) -> io::Result<Self> {
        let p = path.clone();
        let p = Path::new(&p);
        Ok(MusicTrack {
            path: path.into(),
            ext: p
                .extension()
                .ok_or_else(|| io::Error::from(io::ErrorKind::Unsupported))?
                .to_str()
                .unwrap()
                .to_string(),
        })
    }

    /// Returns the `FormatReader` provided by Symphonia
    pub fn get_format(&self) -> Result<Box<dyn FormatReader>, io::Error> {
        let file = fs::File::open(&self.path)?;
        let media_stream = MediaSourceStream::new(Box::new(file), std::default::Default::default());
        let mut hint = Hint::new();
        hint.with_extension(self.ext.as_ref());
        let meta_ops = MetadataOptions::default();
        let fmt_ops = FormatOptions::default();
        let probed = PROBE
            .probe(&hint, media_stream, fmt_ops, meta_ops)
            .map_err(io::Error::other)?;
        Ok(probed)
    }

    pub fn get_meta(&self) -> Result<Metadata, io::Error> {
        let mut format = self.get_format()?;
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| io::Error::other("No audio track"))?;
        let track_id = track.id;
        let length = track
            .time_base
            .zip(track.duration)
            .and_then(|(base, duration)| base.calc_duration(duration))
            .ok_or_else(|| io::Error::other("No audio duration"))?
            .as_secs_f64();
        let time = TrackTime {
            position: 0.0,
            length,
        };

        let mut artist = String::new();
        let mut title = String::new();

        let mut metadata_log = format.metadata();
        while let Some(metadata) = metadata_log.current() {
            let tags = metadata.media.tags.iter().chain(
                metadata
                    .per_track
                    .iter()
                    .filter(|metadata| metadata.track_id == u64::from(track_id))
                    .flat_map(|metadata| metadata.metadata.tags.iter()),
            );
            for tag in tags {
                match &tag.std {
                    Some(StandardTag::Artist(value)) => artist = value.to_string(),
                    Some(StandardTag::TrackTitle(value)) => title = value.to_string(),
                    _ => {}
                }
            }
            if metadata_log.pop().is_none() {
                break;
            }
        }
        if title.is_empty() || artist.is_empty() {
            if let Ok(tag) = Tag::read_from_path(&self.path) {
                if title.is_empty() {
                    if let Some(t) = tag.title() {
                        title = t.to_string();
                    }
                }
                if artist.is_empty() {
                    if let Some(a) = tag.artist() {
                        artist = a;
                    }
                }
            }
        }

        if title.is_empty() {
            title = remove_ext(&self.path);
        }

        title.shrink_to_fit();
        artist.shrink_to_fit();

        Ok(Metadata {
            time,
            artist,
            title,
        })
    }

    pub fn get_length(&self) -> Result<TrackTime, io::Error> {
        let format = self.get_format()?;
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| io::Error::other("No audio track"))?;
        let length = track
            .time_base
            .zip(track.duration)
            .and_then(|(base, duration)| base.calc_duration(duration))
            .ok_or_else(|| io::Error::other("No audio duration"))?
            .as_secs_f64();
        Ok(TrackTime {
            position: 0.0,
            length,
        })
    }
}
