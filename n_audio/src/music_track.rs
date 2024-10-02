use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use crate::{remove_ext, Metadata, TrackTime, PROBE};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia_core::meta::StandardTagKey;

/// The basics where everything is built upon
pub struct MusicTrack {
    path: String,
    name: String,
    ext: String,
}

impl MusicTrack {
    pub fn new<P: AsRef<Path> + AsRef<OsStr> + Clone + Into<String>>(
        path: P,
    ) -> Result<Self, Box<dyn Error>> {
        let p = path.clone();
        let p = Path::new(&p);
        Ok(MusicTrack {
            path: path.into(),
            name: remove_ext(p),
            ext: p
                .extension()
                .ok_or(String::from("no extension"))?
                .to_str()
                .unwrap()
                .to_string(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the `FormatReader` provided by Symphonia
    pub fn get_format(&self) -> Box<dyn FormatReader> {
        let file = fs::read(&self.path).expect("can't read file");
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
        probed.format
    }

    pub fn get_meta(&self) -> Metadata {
        let mut format = self.get_format();
        let track = format.default_track().expect("Can't load tracks");
        let time_base = track.codec_params.time_base.unwrap();

        let duration = track
            .codec_params
            .n_frames
            .map(|frames| track.codec_params.start_ts + frames)
            .unwrap();
        let time = time_base.calc_time(duration);

        let time = TrackTime {
            pos_secs: 0,
            pos_frac: 0.0,
            len_secs: time.seconds,
            len_frac: time.frac,
        };

        let mut artist = String::new();
        let mut title = String::new();

        for tag in format.metadata().current().unwrap().tags() {
            if let Some(StandardTagKey::Artist) = tag.std_key {
                artist = tag.value.to_string();
            } else if let Some(StandardTagKey::TrackTitle) = tag.std_key {
                title = tag.value.to_string();
            }
        }

        Metadata {
            time,
            artist,
            title,
        }
    }

    pub fn get_length(&self) -> TrackTime {
        let mut format = self.get_format();
        let track = format.default_track().expect("Can't load tracks");
        let time_base = track.codec_params.time_base.unwrap();

        let duration = track
            .codec_params
            .n_frames
            .map(|frames| track.codec_params.start_ts + frames)
            .unwrap();
        let time = time_base.calc_time(duration);

        TrackTime {
            pos_secs: 0,
            pos_frac: 0.0,
            len_secs: time.seconds,
            len_frac: time.frac,
        }
    }
}
