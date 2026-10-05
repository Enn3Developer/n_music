//! A track being decoded: opening, seeking, decoding and trimming, and conversion to the
//! output format.

use super::super::convert::Converter;
use super::super::output::OutputFormat;
use super::super::{replay_gain, CODEC_REGISTRY};
use crate::library::track::ReplayGainMode;
use crate::queue::ItemId;
use crate::source::{Locator, Providers};
use std::io;
use std::sync::Arc;
use symphonia::core::audio::Channels;
use symphonia::core::codecs::audio::well_known::CODEC_ID_OPUS;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::units::{Time, TimeBase, Timestamp};

/// Where a track's bytes come from.
#[derive(Clone)]
pub(super) struct PlaybackSource {
    pub(super) providers: Arc<Providers>,
    pub(super) locator: Locator,
}

impl PlaybackSource {
    /// Opens the track from its first byte, from the stream cache's copy when there is one.
    fn open(&self) -> io::Result<Box<dyn FormatReader>> {
        super::super::probe(self.providers.play(&self.locator)?, &self.locator)
    }
}

/// A track being decoded and converted to the output format.
pub(super) struct Decoding {
    pub(super) source: PlaybackSource,
    pub(super) item: ItemId,
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    time_base: TimeBase,
    pub(super) length: f64,
    /// ReplayGain, as a linear factor.
    gain: f32,
    converter: Option<Converter>,
    /// Decoded frames before this are dropped: seeking lands on a packet boundary.
    seek_target: Option<Timestamp>,
    /// Audio was decoded since the last seek, so a new output has to rewind.
    pub(super) decoded: bool,
    interleaved: Vec<f32>,
    /// The decoder leaves encoder delay and padding in (our Opus decoder), so they are
    /// trimmed here, and only where they can be real: Symphonia's Ogg reader flags whole
    /// packets mid-stream as padding when a file's granule positions are off.
    trims: bool,
    /// Encoder delay frames still to drop at the start of the stream.
    start_trim: usize,
    /// The last decoded packet, held back until it is known whether it is the final one,
    /// whose padding is then dropped.
    held: Vec<f32>,
    /// Rate, layout and padding frames of `held`.
    held_spec: Option<(u32, Channels, usize)>,
}

pub(super) enum Decoded {
    Audio,
    Skipped,
    End,
}

impl Decoding {
    pub(super) fn open(
        source: PlaybackSource,
        item: ItemId,
        replay_gain_mode: ReplayGainMode,
    ) -> io::Result<Self> {
        let mut format = source.open()?;
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| io::Error::other("No audio track"))?;
        let track_id = track.id;
        let time_base = track
            .time_base
            .ok_or_else(|| io::Error::other("No audio time base"))?;
        let length = track
            .duration
            .and_then(|duration| time_base.calc_duration(duration))
            .map(|time| time.as_secs_f64())
            .unwrap_or(0.0);
        let params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .ok_or_else(|| io::Error::other("No audio codec parameters"))?;
        let decoder = CODEC_REGISTRY
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .map_err(io::Error::other)?;
        let trims = params.codec == CODEC_ID_OPUS;
        let start_trim = if trims {
            track.delay.unwrap_or(0) as usize
        } else {
            0
        };
        let gain = replay_gain(format.as_mut(), track_id).factor(replay_gain_mode);
        Ok(Self {
            source,
            item,
            format,
            decoder,
            track_id,
            time_base,
            length,
            gain,
            converter: None,
            seek_target: None,
            decoded: false,
            interleaved: vec![],
            trims,
            start_trim,
            held: vec![],
            held_spec: None,
        })
    }

    fn reopen(&mut self) -> io::Result<()> {
        self.format = self.source.open()?;
        self.decoder.reset();
        Ok(())
    }

    fn reset(&mut self) {
        self.decoder.reset();
        if let Some(converter) = &mut self.converter {
            converter.reset();
        }
        self.decoded = false;
        self.start_trim = 0;
        self.held.clear();
        self.held_spec = None;
    }

    /// Seeks to `target`; returns the position it landed on and whether the reader had to be
    /// rebuilt to get there. `rewind` reopens the source first: some demuxers cannot seek
    /// backward without an index, or after EOF.
    pub(super) fn seek(
        &mut self,
        target: Time,
        rewind: bool,
        automatic: bool,
    ) -> io::Result<Option<(f64, bool)>> {
        if rewind {
            self.reopen()?;
        }
        let seeked = self.format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: target,
                track_id: Some(self.track_id),
            },
        );
        if (automatic && seeked.is_err())
            || (rewind
                && seeked
                    .as_ref()
                    .is_ok_and(|seeked| seeked.actual_ts > seeked.required_ts))
        {
            // An unindexed reader may only seek forward past the target packet.
            // Decode and discard from the start in that case, rather than skipping audio.
            self.reopen()?;
            self.reset();
            self.seek_target = Some(
                self.time_base
                    .calc_timestamp(target)
                    .ok_or_else(|| io::Error::other("Seek time out of range"))?,
            );
            return Ok(Some((target.as_secs_f64(), true)));
        }
        match seeked {
            Ok(seeked) => {
                self.reset();
                self.seek_target = Some(seeked.required_ts);
                let position = self
                    .time_base
                    .calc_time(seeked.required_ts)
                    .ok_or_else(|| io::Error::other("Seek time out of range"))?;
                Ok(Some((position.as_secs_f64(), false)))
            }
            Err(err) => {
                log::warn!("Could not seek to {target:?}: {err}");
                Ok(None)
            }
        }
    }

    /// Decodes one packet into `out`, converted to `output`.
    pub(super) fn decode(
        &mut self,
        output: OutputFormat,
        out: &mut Vec<f32>,
    ) -> io::Result<Decoded> {
        let packet = match self.format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => return Ok(Decoded::End),
            Err(err) => return Err(io::Error::other(err)),
        };
        if packet.track_id != self.track_id {
            return Ok(Decoded::Skipped);
        }
        while !self.format.metadata().is_latest() {
            self.format.metadata().pop();
        }
        let decoded = match self.decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(symphonia::core::errors::Error::DecodeError(_)) => return Ok(Decoded::Skipped),
            Err(err) => return Err(io::Error::other(err)),
        };
        let packet_start = packet.pts.saturating_add(packet.trim_start);
        let skip_frames = match self.seek_target {
            Some(target) if packet_start < target => {
                let skip = self
                    .time_base
                    .calc_duration(target.abs_delta(packet_start))
                    .ok_or_else(|| io::Error::other("Seek duration out of range"))?;
                (skip.as_secs_f64() * decoded.spec().rate() as f64).round() as usize
            }
            _ => 0,
        };
        if skip_frames >= decoded.frames() {
            return Ok(Decoded::Skipped);
        }
        self.seek_target = None;
        let rate = decoded.spec().rate();
        let channels = decoded.spec().channels().clone();
        let frames = decoded.frames();
        decoded.copy_to_vec_interleaved(&mut self.interleaved);
        let mut start = skip_frames;
        let mut trim_end = 0;
        if self.trims {
            let trim = self.start_trim.min(frames);
            self.start_trim -= trim;
            start = start.max(trim);
            trim_end = self
                .time_base
                .calc_duration(packet.trim_end)
                .map_or(0, |time| {
                    (time.as_secs_f64() * rate as f64).round() as usize
                });
        }
        if start >= frames {
            return Ok(Decoded::Skipped);
        }
        self.interleaved.drain(..start * channels.count());
        self.release(output, out, false)?;
        std::mem::swap(&mut self.held, &mut self.interleaved);
        self.held_spec = Some((rate, channels, trim_end));
        self.decoded = true;
        Ok(Decoded::Audio)
    }

    /// Converts the held-back packet into `out`; `last` drops its padding.
    fn release(&mut self, output: OutputFormat, out: &mut Vec<f32>, last: bool) -> io::Result<()> {
        let Some((rate, channels, trim_end)) = self.held_spec.take() else {
            return Ok(());
        };
        let mut len = self.held.len();
        if last {
            len = len.saturating_sub(trim_end * channels.count());
        }
        if !self
            .converter
            .as_ref()
            .is_some_and(|converter| converter.fits(rate, &channels, output))
        {
            self.converter =
                Some(Converter::new(rate, &channels, output, self.gain).map_err(io::Error::other)?);
        }
        if let Some(converter) = &mut self.converter {
            converter.push(&self.held[..len], out);
        }
        self.held.clear();
        Ok(())
    }

    /// Converts everything still held at the end of the track.
    pub(super) fn finish(&mut self, output: OutputFormat, out: &mut Vec<f32>) -> io::Result<()> {
        self.release(output, out, true)?;
        if let Some(converter) = &mut self.converter {
            converter.finish(out);
        }
        Ok(())
    }
}
