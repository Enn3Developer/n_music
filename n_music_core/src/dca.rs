use audiopus::SampleRate;
use serde::{Deserialize, Serialize};
use std::io::{Seek, SeekFrom};
use symphonia::core::{
    audio::sample::SampleFormat,
    codecs::{
        audio::{well_known::CODEC_ID_OPUS, AudioCodecParameters},
        CodecParameters,
    },
    common::FourCc,
    errors::{self as symph_err, Error as SymphError, Result as SymphResult, SeekErrorKind},
    formats::prelude::*,
    formats::probe::{ProbeFormatData, ProbeableFormat, Score, Scoreable},
    io::{MediaSource, MediaSourceStream, ReadBytes, ScopedStream, SeekBuffered},
    meta::{
        Metadata as SymphMetadata, MetadataBuilder, MetadataId, MetadataInfo, MetadataLog,
        RawValue, StandardTag, Tag,
    },
    packet::Packet,
    units::{Duration, TimeBase, Timestamp},
};

// Original code from the Songbird project

#[derive(Debug, Deserialize, Serialize)]
pub struct DcaMetadata {
    pub dca: DcaInfo,
    pub opus: Opus,
    pub info: Option<Info>,
    pub origin: Option<Origin>,
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DcaInfo {
    pub version: u64,
    pub tool: Tool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Tool {
    pub name: String,
    pub version: String,
    pub url: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Opus {
    pub mode: String,
    pub sample_rate: u32,
    pub frame_size: u64,
    pub abr: Option<u64>,
    pub vbr: bool,
    pub channels: u8,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Info {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub cover: Option<String>,
    pub comments: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Origin {
    pub source: Option<String>,
    pub abr: Option<u64>,
    pub channels: Option<u8>,
    pub encoding: Option<String>,
    pub url: Option<String>,
}

const FORMAT_INFO: FormatInfo = FormatInfo {
    format: FormatId::new(FourCc::new(*b"DCA ")),
    short_name: "dca",
    long_name: "DCA[0/1] Opus Wrapper",
};

impl Scoreable for DcaReader<'_> {
    fn score(_source: ScopedStream<&mut MediaSourceStream<'_>>) -> SymphResult<Score> {
        Ok(Score::Supported(255))
    }
}

impl<'s> ProbeableFormat<'s> for DcaReader<'_> {
    fn try_probe_new(
        source: MediaSourceStream<'s>,
        options: FormatOptions,
    ) -> SymphResult<Box<dyn FormatReader + 's>> {
        Ok(Box::new(DcaReader::try_new(source, options)?))
    }

    fn probe_data() -> &'static [ProbeFormatData] {
        &[symphonia_core::support_format!(
            FORMAT_INFO,
            &["dca"],
            &[],
            &[b"DCA1"]
        )]
    }
}

struct SeekAccel {
    frame_offsets: Vec<(Timestamp, u64)>,
    seek_index_fill_period_ms: u16,
    next_ts: Timestamp,
}

impl SeekAccel {
    fn new(options: &FormatOptions, first_frame_byte_pos: u64) -> Self {
        let per_s = options.seek_index_fill_period_ms;
        let next_ts = u64::from(per_s) * 48;

        Self {
            frame_offsets: vec![(Timestamp::ZERO, first_frame_byte_pos)],
            seek_index_fill_period_ms: per_s,
            next_ts: Timestamp::new(next_ts as i64),
        }
    }

    fn update(&mut self, ts: Timestamp, pos: u64) {
        if ts >= self.next_ts {
            self.next_ts = self.next_ts.saturating_add(Duration::from(
                u64::from(self.seek_index_fill_period_ms) * 48,
            ));
            self.frame_offsets.push((ts, pos));
        }
    }

    fn get_seek_pos(&self, ts: Timestamp) -> (Timestamp, u64) {
        let index = self.frame_offsets.partition_point(|&(o_ts, _)| o_ts <= ts) - 1;
        self.frame_offsets[index]
    }
}

/// [DCA\[0/1\]](https://github.com/bwmarrin/dca) Format reader for Symphonia.
pub struct DcaReader<'s> {
    source: MediaSourceStream<'s>,
    media_info: MediaInfo,
    track: Option<Track>,
    metas: MetadataLog,
    seek_accel: SeekAccel,
    curr_ts: Timestamp,
    max_ts: Option<Timestamp>,
    held_packet: Option<Packet>,
}

impl<'s> DcaReader<'s> {
    fn try_new(mut source: MediaSourceStream<'s>, mut options: FormatOptions) -> SymphResult<Self> {
        // Read in the magic number to verify it's a DCA file.
        let magic = source.read_quad_bytes()?;

        let read_meta = match &magic {
            b"DCA1" => true,
            _ if &magic[..3] == b"DCA" => {
                return symph_err::unsupported_error("unsupported DCA version");
            }
            _ => {
                source.seek_buffered_rel(-4);
                false
            }
        };

        let mut codec_params = AudioCodecParameters::new();

        codec_params
            .for_codec(CODEC_ID_OPUS)
            .with_max_frames_per_packet(1)
            .with_sample_rate(48000)
            .with_sample_format(SampleFormat::F32);

        let mut metas = options.external_data.metadata.take().unwrap_or_default();

        if read_meta {
            let size = source.read_u32()?;

            // Sanity check
            if (size as i32) < 2 {
                return symph_err::decode_error("missing DCA1 metadata block");
            }

            let mut raw_json = source.read_boxed_slice_exact(size as usize)?;

            // NOTE: must be mut for simd-json.
            #[allow(clippy::unnecessary_mut_passed)]
            let metadata: DcaMetadata = serde_json::from_slice::<DcaMetadata>(&mut raw_json)
                .map_err(|_| SymphError::DecodeError("malformed DCA1 metadata block"))?;

            let mut revision = MetadataBuilder::new(MetadataInfo {
                metadata: MetadataId::new(FourCc::new(*b"DCA ")),
                short_name: "dca",
                long_name: "DCA metadata",
            });

            if let Some(info) = metadata.info {
                if let Some(t) = info.title {
                    revision.add_tag(Tag::new_from_parts(
                        "title",
                        RawValue::from(t.clone()),
                        Some(StandardTag::TrackTitle(t.into())),
                    ));
                }
                if let Some(t) = info.album {
                    revision.add_tag(Tag::new_from_parts(
                        "album",
                        RawValue::from(t.clone()),
                        Some(StandardTag::Album(t.into())),
                    ));
                }
                if let Some(t) = info.artist {
                    revision.add_tag(Tag::new_from_parts(
                        "artist",
                        RawValue::from(t.clone()),
                        Some(StandardTag::Artist(t.into())),
                    ));
                }
                if let Some(t) = info.genre {
                    revision.add_tag(Tag::new_from_parts(
                        "genre",
                        RawValue::from(t.clone()),
                        Some(StandardTag::Genre(t.into())),
                    ));
                }
                if let Some(t) = info.comments {
                    revision.add_tag(Tag::new_from_parts(
                        "comments",
                        RawValue::from(t.clone()),
                        Some(StandardTag::Comment(t.into())),
                    ));
                }
                if let Some(_t) = info.cover {
                    // TODO: Add visual, figure out MIME types.
                }
            }

            if let Some(origin) = metadata.origin {
                if let Some(t) = origin.url {
                    revision.add_tag(Tag::new_from_parts(
                        "url",
                        RawValue::from(t.clone()),
                        Some(StandardTag::Url(t.into())),
                    ));
                }
            }

            metas.push(revision.build());
        }

        let bytes_read = source.pos();

        Ok(Self {
            source,
            media_info: {
                let mut info = MediaInfo::default();
                info.time_base = TimeBase::try_from_recip(48000);
                info
            },
            track: Some({
                let mut track = Track::new(0);
                track.with_codec_params(CodecParameters::Audio(codec_params));
                track
            }),
            metas,
            seek_accel: SeekAccel::new(&options, bytes_read),
            curr_ts: Timestamp::ZERO,
            max_ts: None,
            held_packet: None,
        })
    }
}

impl FormatReader for DcaReader<'_> {
    fn format_info(&self) -> &FormatInfo {
        &FORMAT_INFO
    }

    fn media_info(&self) -> &MediaInfo {
        &self.media_info
    }

    fn metadata(&mut self) -> SymphMetadata<'_> {
        self.metas.metadata()
    }

    fn seek(&mut self, _mode: SeekMode, to: SeekTo) -> SymphResult<SeekedTo> {
        let can_backseek = self.source.is_seekable();

        let track = if self.track.is_none() {
            return symph_err::seek_error(SeekErrorKind::Unseekable);
        } else {
            self.track.as_ref().unwrap()
        };

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

        if ts.is_negative() {
            return symph_err::seek_error(SeekErrorKind::OutOfRange);
        }
        if let Some(max_ts) = self.max_ts {
            if ts > max_ts {
                return symph_err::seek_error(SeekErrorKind::OutOfRange);
            }
        }

        let backseek_needed = self.curr_ts > ts;

        if backseek_needed && !can_backseek {
            return symph_err::seek_error(SeekErrorKind::ForwardOnly);
        }

        let (accel_seek_ts, accel_seek_pos) = self.seek_accel.get_seek_pos(ts);

        if backseek_needed || accel_seek_pos > self.source.pos() {
            self.source.seek(SeekFrom::Start(accel_seek_pos))?;
            self.curr_ts = accel_seek_ts;
            self.held_packet = None;
        }

        while let Some(pkt) = self.next_packet()? {
            let pts = pkt.pts;
            let dur = pkt.dur;
            let track_id = pkt.track_id;

            if (pts..pts.saturating_add(dur)).contains(&ts) {
                self.held_packet = Some(pkt);
                return Ok(SeekedTo {
                    track_id,
                    required_ts: ts,
                    actual_ts: pts,
                });
            }
        }

        symph_err::seek_error(SeekErrorKind::OutOfRange)
    }

    fn tracks(&self) -> &[Track] {
        // DCA tracks can hold only one track by design.
        // Of course, a zero-length file is technically allowed,
        // in which case no track.
        if let Some(track) = self.track.as_ref() {
            std::slice::from_ref(track)
        } else {
            &[]
        }
    }

    fn next_packet(&mut self) -> SymphResult<Option<Packet>> {
        if let Some(pkt) = self.held_packet.take() {
            return Ok(Some(pkt));
        }

        let frame_pos = self.source.pos();

        let first = match self.source.read_byte() {
            Ok(byte) => byte,
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                self.max_ts = Some(self.curr_ts);
                return Ok(None);
            }
            Err(err) => return Err(err.into()),
        };
        let p_len = i16::from_le_bytes([first, self.source.read_byte()?]);

        if p_len < 0 {
            return symph_err::decode_error("DCA frame header had a negative length.");
        }

        let buf = self.source.read_boxed_slice_exact(p_len as usize)?;

        let checked_buf = buf[..].try_into().or_else(|_| {
            symph_err::decode_error("Packet was not a valid Opus Packet: too large for audiopus.")
        })?;

        let sample_ct =
            audiopus::packet::nb_samples(checked_buf, SampleRate::Hz48000).or_else(|_| {
                symph_err::decode_error(
                    "Packet was not a valid Opus packet: couldn't read sample count.",
                )
            })? as u64;

        let out = Packet::new(0, self.curr_ts, Duration::from(sample_ct), buf);

        self.seek_accel.update(self.curr_ts, frame_pos);

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
