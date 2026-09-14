use std::io::{Seek, SeekFrom};
use symphonia::core::{
    audio::layouts,
    codecs::{
        audio::{well_known::CODEC_ID_PCM_F32LE, AudioCodecParameters},
        CodecParameters,
    },
    common::FourCc,
    errors::{self as symph_err, Error as SymphError, Result as SymphResult, SeekErrorKind},
    formats::prelude::*,
    formats::probe::{ProbeFormatData, ProbeableFormat, Score, Scoreable},
    io::{MediaSource, MediaSourceStream, ReadBytes, ScopedStream, SeekBuffered},
    meta::{Metadata as SymphMetadata, MetadataLog},
    packet::Packet,
    units::{Duration, TimeBase, Timestamp},
};

// Original code from the Songbird project

const FORMAT_INFO: FormatInfo = FormatInfo {
    format: FormatId::new(FourCc::new(*b"SBRW")),
    short_name: "raw",
    long_name: "Raw arbitrary-length f32 audio container.",
};

impl Scoreable for RawReader<'_> {
    fn score(_source: ScopedStream<&mut MediaSourceStream<'_>>) -> SymphResult<Score> {
        Ok(Score::Supported(255))
    }
}

impl<'s> ProbeableFormat<'s> for RawReader<'_> {
    fn try_probe_new(
        source: MediaSourceStream<'s>,
        options: FormatOptions,
    ) -> SymphResult<Box<dyn FormatReader + 's>> {
        Ok(Box::new(RawReader::try_new(source, options)?))
    }

    fn probe_data() -> &'static [ProbeFormatData] {
        &[symphonia_core::support_format!(
            FORMAT_INFO,
            &["rawf32"],
            &[],
            &[b"SbirdRaw"]
        )]
    }
}

/// Symphonia support for a simple container for raw f32-PCM data of unknown duration.
///
/// Contained files have a simple header:
/// * the 8-byte signature `b"SbirdRaw"`,
/// * the sample rate, as a little-endian `u32`,
/// * the channel count, as a little-endian `u32`.
///
/// The remainder of the file is interleaved little-endian `f32` samples.
pub struct RawReader<'s> {
    source: MediaSourceStream<'s>,
    media_info: MediaInfo,
    track: Track,
    meta: MetadataLog,
    curr_ts: Timestamp,
    max_ts: Option<Timestamp>,
}

impl<'s> RawReader<'s> {
    fn try_new(mut source: MediaSourceStream<'s>, options: FormatOptions) -> SymphResult<Self> {
        let mut magic = [0u8; 8];
        ReadBytes::read_buf_exact(&mut source, &mut magic[..])?;

        if &magic != b"SbirdRaw" {
            source.seek_buffered_rel(-(magic.len() as isize));
            return symph_err::decode_error("rawf32: illegal magic byte sequence.");
        }

        let sample_rate = source.read_u32()?;
        let n_chans = source.read_u32()?;
        if sample_rate < 50 {
            return symph_err::decode_error("rawf32: invalid sample rate");
        }

        let chans = match n_chans {
            1 => layouts::CHANNEL_LAYOUT_MONO,
            2 => layouts::CHANNEL_LAYOUT_STEREO,
            _ => {
                return symph_err::decode_error(
                    "rawf32: channel layout is not stereo or mono for fmt_pcm",
                );
            }
        };

        let mut codec_params = AudioCodecParameters::new();

        codec_params
            .for_codec(CODEC_ID_PCM_F32LE)
            .with_bits_per_coded_sample((std::mem::size_of::<f32>() as u32) * 8)
            .with_bits_per_sample((std::mem::size_of::<f32>() as u32) * 8)
            .with_sample_rate(sample_rate)
            .with_sample_format(symphonia_core::audio::sample::SampleFormat::F32)
            .with_max_frames_per_packet(sample_rate as u64 / 50)
            .with_channels(chans);

        Ok(Self {
            source,
            media_info: {
                let mut info = MediaInfo::default();
                info.time_base = TimeBase::try_from_recip(sample_rate);
                info
            },
            track: {
                let mut track = Track::new(0);
                track.with_codec_params(CodecParameters::Audio(codec_params));
                track
            },
            meta: options.external_data.metadata.unwrap_or_default(),
            curr_ts: Timestamp::ZERO,
            max_ts: None,
        })
    }
}

impl FormatReader for RawReader<'_> {
    fn format_info(&self) -> &FormatInfo {
        &FORMAT_INFO
    }

    fn media_info(&self) -> &MediaInfo {
        &self.media_info
    }

    fn metadata(&mut self) -> SymphMetadata<'_> {
        self.meta.metadata()
    }

    fn seek(&mut self, _mode: SeekMode, to: SeekTo) -> SymphResult<SeekedTo> {
        let can_backseek = self.source.is_seekable();

        let track = &self.track;
        let rate = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .and_then(|params| params.sample_rate);
        let ts = match to {
            SeekTo::Time { time, .. } => {
                if let Some(rate) = rate {
                    TimeBase::try_from_recip(rate)
                        .and_then(|base| base.calc_timestamp(time))
                        .ok_or(SymphError::SeekError(SeekErrorKind::OutOfRange))?
                } else {
                    return symph_err::seek_error(SeekErrorKind::Unseekable);
                }
            }
            SeekTo::Timestamp { ts, .. } => ts,
        };

        if let Some(max_ts) = self.max_ts {
            if ts > max_ts {
                return symph_err::seek_error(SeekErrorKind::OutOfRange);
            }
        }

        let backseek_needed = self.curr_ts > ts;

        if backseek_needed && !can_backseek {
            return symph_err::seek_error(SeekErrorKind::ForwardOnly);
        }

        let chan_count = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .unwrap()
            .channels
            .as_ref()
            .expect("Channel count is built into format.")
            .count() as u64;

        let seek_pos = u64::try_from(ts.get())
            .ok()
            .and_then(|ts| ts.checked_mul(chan_count))
            .and_then(|samples| samples.checked_mul(std::mem::size_of::<f32>() as u64))
            .and_then(|bytes| bytes.checked_add(16))
            .ok_or(SymphError::SeekError(SeekErrorKind::OutOfRange))?;

        self.source.seek(SeekFrom::Start(seek_pos))?;
        self.curr_ts = ts;

        Ok(SeekedTo {
            track_id: track.id,
            required_ts: ts,
            actual_ts: ts,
        })
    }

    fn tracks(&self) -> &[Track] {
        std::slice::from_ref(&self.track)
    }

    fn next_packet(&mut self) -> SymphResult<Option<Packet>> {
        let track = &self.track;
        let rate = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .unwrap()
            .sample_rate
            .expect("Sample rate is built into format.") as usize;

        let chan_count = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .unwrap()
            .channels
            .as_ref()
            .expect("Channel count is built into format.")
            .count();

        let sample_unit = std::mem::size_of::<f32>() * chan_count;

        // Aim for 20ms (50Hz).
        let buf = self.source.read_boxed_slice((rate / 50) * sample_unit)?;

        if buf.is_empty() {
            self.max_ts = Some(self.curr_ts);
            return Ok(None);
        }
        if buf.len() % sample_unit != 0 {
            return symph_err::decode_error("rawf32: incomplete audio frame");
        }
        let sample_ct = (buf.len() / sample_unit) as u64;
        let out = Packet::new(0, self.curr_ts, Duration::from(sample_ct), buf);

        self.curr_ts = self
            .curr_ts
            .checked_add(Duration::from(sample_ct))
            .ok_or(SymphError::DecodeError("Timestamp overflow"))?;

        Ok(Some(out))
    }

    fn into_inner<'s>(self: Box<Self>) -> MediaSourceStream<'s>
    where
        Self: 's,
    {
        self.source
    }
}
