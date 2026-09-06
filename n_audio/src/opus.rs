use audiopus::{
    coder::{Decoder as AudiopusDecoder, GenericCtl},
    Channels, Error as OpusError, ErrorCode, SampleRate,
};
use symphonia_core::{
    audio::{
        layouts, AsGenericAudioBufferRef, AudioBuffer, AudioMut, AudioSpec, GenericAudioBufferRef,
    },
    codecs::{
        audio::{
            well_known::CODEC_ID_OPUS, AudioCodecParameters, AudioDecoder, AudioDecoderOptions,
            FinalizeResult,
        },
        registry::{RegisterableAudioDecoder, SupportedAudioCodec},
        CodecInfo,
    },
    errors::{decode_error, Result as SymphResult},
    packet::PacketRef,
};

const CODEC_INFO: CodecInfo = CodecInfo {
    short_name: "opus",
    long_name: "libopus (1.3+, audiopus)",
    profiles: &[],
};

// Original code from the Songbird project

/// Opus decoder for symphonia, based on libopus v1.3 (via [`audiopus`]).
pub struct OpusDecoder {
    inner: AudiopusDecoder,
    params: AudioCodecParameters,
    buf: AudioBuffer<f32>,
    rawbuf: Vec<f32>,
    gapless: bool,
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
    fn decode_inner(&mut self, packet: &PacketRef<'_>) -> SymphResult<()> {
        let s_ct = loop {
            let pkt = if packet.data.is_empty() {
                None
            } else if let Ok(checked_pkt) = packet.data.try_into() {
                Some(checked_pkt)
            } else {
                return decode_error("Opus packet was too large (greater than i32::MAX bytes).");
            };
            let out_space = (&mut self.rawbuf[..]).try_into().expect("The following logic expands this buffer safely below i32::MAX, and we throw our own error.");

            match self.inner.decode_float(pkt, out_space, false) {
                Ok(v) => break v,
                Err(OpusError::Opus(ErrorCode::BufferTooSmall)) => {
                    // double the buffer size
                    // correct behav would be to mirror the decoder logic in the udp_rx set.
                    let new_size = (self.rawbuf.len() * 2).min(i32::MAX as usize);
                    if new_size == self.rawbuf.len() {
                        return decode_error(
                            "Opus frame too big: cannot expand opus frame decode buffer any further.",
                        );
                    }

                    self.rawbuf.resize(new_size, 0.0);
                    self.buf = AudioBuffer::new(
                        AudioSpec::new(48000, layouts::CHANNEL_LAYOUT_STEREO),
                        self.rawbuf.len() / 2,
                    );
                }
                Err(_) => {
                    return decode_error("Opus decode error: see 'tracing' logs.");
                }
            }
        };

        self.buf.clear();
        self.buf.resize_uninit(s_ct);

        // Forcibly assuming stereo, for now.
        for ch in 0..2 {
            let iter = self.rawbuf.chunks_exact(2).map(|chunk| chunk[ch]);
            for (tgt, src) in self.buf.plane_mut(ch).unwrap().iter_mut().zip(iter) {
                *tgt = src;
            }
        }

        if self.gapless {
            self.buf.trim(
                packet.trim_start.get() as usize,
                packet.trim_end.get() as usize,
            );
        }
        Ok(())
    }
}

impl OpusDecoder {
    fn try_new(params: &AudioCodecParameters, options: &AudioDecoderOptions) -> SymphResult<Self> {
        let inner = AudiopusDecoder::new(SampleRate::Hz48000, Channels::Stereo).unwrap();

        let mut params = params.clone();
        params.with_sample_rate(48000);

        Ok(Self {
            inner,
            params,
            buf: AudioBuffer::new(
                AudioSpec::new(48000, layouts::CHANNEL_LAYOUT_STEREO),
                48000 / 50,
            ),
            gapless: options.gapless,
            rawbuf: vec![0.0f32; 2 * (48000 / 50)],
        })
    }
}

impl RegisterableAudioDecoder for OpusDecoder {
    fn try_registry_new(
        params: &AudioCodecParameters,
        options: &AudioDecoderOptions,
    ) -> SymphResult<Box<dyn AudioDecoder>> {
        Ok(Box::new(Self::try_new(params, options)?))
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
        _ = self.inner.reset_state();
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
