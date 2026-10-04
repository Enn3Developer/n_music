use audiopus::{
    coder::{Decoder as AudiopusDecoder, GenericCtl},
    Channels as OpusChannels, SampleRate,
};
use symphonia_core::{
    audio::{
        layouts, AsGenericAudioBufferRef, AudioBuffer, AudioMut, AudioSpec, Channels,
        GenericAudioBufferRef,
    },
    codecs::{
        audio::{
            well_known::CODEC_ID_OPUS, AudioCodecParameters, AudioDecoder, AudioDecoderOptions,
            FinalizeResult,
        },
        registry::{RegisterableAudioDecoder, SupportedAudioCodec},
        CodecInfo,
    },
    errors::{decode_error, unsupported_error, Error as SymphError, Result as SymphResult},
    packet::PacketRef,
};

const CODEC_INFO: CodecInfo = CodecInfo {
    short_name: "opus",
    long_name: "libopus (1.3+, audiopus)",
    profiles: &[],
};

const SAMPLE_RATE: u32 = 48_000;

const MAX_FRAMES_PER_PACKET: usize = SAMPLE_RATE as usize * 120 / 1000;

const MIN_FRAME: usize = SAMPLE_RATE as usize * 5 / 2000;

const DEFAULT_PLC_FRAMES: usize = SAMPLE_RATE as usize / 50;

struct OpusHead {
    channels: u8,
    output_gain: i16,
    mapping_family: u8,
    stream_count: u8,
}

impl OpusHead {
    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 19 || &data[..8] != b"OpusHead" {
            return None;
        }

        let mapping_family = data[18];
        let stream_count = if mapping_family == 0 {
            1
        } else {
            *data.get(19)?
        };

        Some(Self {
            channels: data[9],
            output_gain: i16::from_le_bytes([data[16], data[17]]),
            mapping_family,
            stream_count,
        })
    }
}

// Original code from the Songbird project

/// Opus decoder for symphonia, based on libopus v1.3 (via [`audiopus`]).
pub struct OpusDecoder {
    inner: AudiopusDecoder,
    params: AudioCodecParameters,
    buf: AudioBuffer<f32>,
    rawbuf: Box<[f32]>,
    channels: usize,
}

/// # SAFETY
/// The underlying Opus decoder (currently) requires only a `&self` parameter
/// to decode given packets, which is likely a mistaken decision.
///
/// This struct makes stronger assumptions and only touches FFI decoder state with a
/// `&mut self`, preventing data races via `&OpusDecoder` as required by `impl Sync`.
/// No access to other internal state relies on unsafety or crosses FFI.
unsafe impl Sync for OpusDecoder {}

impl OpusDecoder {
    fn try_new(params: &AudioCodecParameters) -> SymphResult<Self> {
        let head = params.extra_data.as_deref().and_then(OpusHead::parse);

        if let Some(head) = &head {
            if head.stream_count != 1 || head.channels > 2 {
                log::warn!(
                    "Multistream Opus is not supported (mapping family {}, {} channels, {} streams)",
                    head.mapping_family,
                    head.channels,
                    head.stream_count
                );
                return unsupported_error("Multistream (surround) Opus is not supported");
            }
        }

        let channel_count = head
            .as_ref()
            .map(|head| usize::from(head.channels))
            .or_else(|| params.channels.as_ref().map(Channels::count))
            .unwrap_or(2);

        let (opus_channels, layout) = match channel_count {
            1 => (OpusChannels::Mono, layouts::CHANNEL_LAYOUT_MONO),
            2 => (OpusChannels::Stereo, layouts::CHANNEL_LAYOUT_STEREO),
            n => {
                return unsupported_error(if n == 0 {
                    "Opus stream declares zero channels"
                } else {
                    "Opus streams with more than 2 channels are not supported"
                })
            }
        };

        let inner = AudiopusDecoder::new(SampleRate::Hz48000, opus_channels).map_err(|error| {
            log::error!("Could not initialize the native Opus decoder: {error:?}");
            SymphError::DecodeError("Could not initialize the native Opus decoder")
        })?;

        if let Some(gain) = head
            .as_ref()
            .map(|head| head.output_gain)
            .filter(|&g| g != 0)
        {
            if let Err(error) = inner.set_gain(i32::from(gain)) {
                log::warn!("Could not apply Opus output gain of {gain} Q7.8 dB: {error:?}");
            }
        }

        let mut params = params.clone();
        params
            .with_sample_rate(SAMPLE_RATE)
            .with_channels(layout.clone());

        Ok(Self {
            inner,
            params,
            buf: AudioBuffer::new(AudioSpec::new(SAMPLE_RATE, layout), MAX_FRAMES_PER_PACKET),
            rawbuf: vec![0.0; MAX_FRAMES_PER_PACKET * channel_count].into_boxed_slice(),
            channels: channel_count,
        })
    }

    fn plc_frames(&self, packet: &PacketRef<'_>) -> usize {
        let frames = match packet.dur.get() as usize {
            0 => self
                .inner
                .last_packet_duration()
                .map(|dur| dur as usize)
                .unwrap_or(DEFAULT_PLC_FRAMES),
            dur => dur,
        };

        (frames / MIN_FRAME * MIN_FRAME).clamp(MIN_FRAME, MAX_FRAMES_PER_PACKET)
    }

    fn decode_inner(&mut self, packet: &PacketRef<'_>) -> SymphResult<()> {
        let (input, out_len) = if packet.data.is_empty() {
            (None, self.plc_frames(packet) * self.channels)
        } else {
            let Ok(pkt) = packet.data.try_into() else {
                return decode_error("Opus packet was too large (greater than i32::MAX bytes).");
            };
            (Some(pkt), self.rawbuf.len())
        };

        let out = (&mut self.rawbuf[..out_len])
            .try_into()
            .expect("Opus scratch buffer is bounded far below i32::MAX");

        let frames = self
            .inner
            .decode_float(input, out, false)
            .map_err(|error| {
                log::debug!("Opus packet decode failed: {error:?}");
                SymphError::DecodeError("Opus packet decode failed")
            })?;

        self.buf.clear();
        self.buf.resize_uninit(frames);
        self.buf
            .copy_from_slice_interleaved(&&self.rawbuf[..frames * self.channels]);
        Ok(())
    }
}

impl RegisterableAudioDecoder for OpusDecoder {
    fn try_registry_new(
        params: &AudioCodecParameters,
        _options: &AudioDecoderOptions,
    ) -> SymphResult<Box<dyn AudioDecoder>> {
        Ok(Box::new(Self::try_new(params)?))
    }

    fn supported_codecs() -> &'static [SupportedAudioCodec] {
        &[SupportedAudioCodec {
            id: CODEC_ID_OPUS,
            info: CODEC_INFO,
        }]
    }
}

impl AudioDecoder for OpusDecoder {
    fn codec_info(&self) -> &CodecInfo {
        &CODEC_INFO
    }

    fn reset(&mut self) {
        if let Err(error) = self.inner.reset_state() {
            log::warn!("Could not reset the Opus decoder: {error:?}");
        }
    }

    fn codec_params(&self) -> &AudioCodecParameters {
        &self.params
    }

    fn decode_ref(&mut self, packet: &PacketRef<'_>) -> SymphResult<GenericAudioBufferRef<'_>> {
        if let Err(e) = self.decode_inner(packet) {
            self.buf.clear();
            Err(e)
        } else {
            Ok(self.buf.as_generic_audio_buffer_ref())
        }
    }

    fn finalize(&mut self) -> FinalizeResult {
        FinalizeResult::default()
    }

    fn last_decoded(&self) -> GenericAudioBufferRef<'_> {
        self.buf.as_generic_audio_buffer_ref()
    }
}
