use crate::{remove_ext, Metadata, TrackTime, PROBE};
use multitag::Tag;
use std::ffi::OsStr;
use std::io::Cursor;
use std::path::Path;
use std::{fs, io};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia_core::meta::StandardTagKey;

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
                .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidFilename))?
                .to_str()
                .unwrap()
                .to_string(),
        })
    }

    /// Returns the `FormatReader` provided by Symphonia
    pub fn get_format(&self) -> Result<Box<dyn FormatReader>, io::Error> {
        let file = fs::read(&self.path)?;
        let media_stream = MediaSourceStream::new(
            Box::new(Cursor::new(file)),
            std::default::Default::default(),
        );
        let mut hint = Hint::new();
        hint.with_extension(self.ext.as_ref());
        let meta_ops = MetadataOptions::default();
        let fmt_ops = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let probed = PROBE
            .format(&hint, media_stream, &fmt_ops, &meta_ops)
            .expect("Format not supported");
        Ok(probed.format)
    }

    pub fn get_meta(&self) -> Result<Metadata, io::Error> {
        let mut format = self.get_format()?;
        let track = format.default_track().expect("Can't load tracks");
        let time_base = track.codec_params.time_base.unwrap();

        let duration = track
            .codec_params
            .n_frames
            .map(|frames| track.codec_params.start_ts + frames)
            .unwrap();
        let time = time_base.calc_time(duration);

        let time = TrackTime {
            position: 0.0,
            length: time.seconds as f64 + time.frac,
        };

        let mut artist = String::new();
        let mut title = String::new();

        if let Some(metadata) = format.metadata().skip_to_latest() {
            for tag in metadata.tags() {
                if let Some(StandardTagKey::Artist) = tag.std_key {
                    artist = tag.value.to_string();
                } else if let Some(StandardTagKey::TrackTitle) = tag.std_key {
                    title = tag.value.to_string();
                }
            }
        } else if let Ok(tag) = Tag::read_from_path(&self.path) {
            if let Some(t) = tag.title() {
                title = t.to_string();
            }
            if let Some(a) = tag.artist() {
                artist = a;
            }
        }

        if title.is_empty() {
            title = remove_ext(&self.path);
        }

        Ok(Metadata {
            time,
            artist,
            title,
        })
    }

    pub fn get_length(&self) -> Result<TrackTime, io::Error> {
        let format = self.get_format()?;
        let track = format.default_track().expect("Can't load tracks");
        let time_base = track.codec_params.time_base.unwrap();

        let duration = track
            .codec_params
            .n_frames
            .map(|frames| track.codec_params.start_ts + frames)
            .unwrap();
        let time = time_base.calc_time(duration);

        Ok(TrackTime {
            position: 0.0,
            length: time.seconds as f64 + time.frac,
        })
    }
}
