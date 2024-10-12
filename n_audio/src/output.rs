//! Platform-dependant Audio Outputs

/// This is a modified version of [symphonia-play's `output.rs`](https://github.com/pdeljanov/Symphonia/blob/master/symphonia-play/src/output.rs)
/// It was originally made by [Philip Deljanov](https://github.com/pdeljanov)
/// Modifications: support for volume (for all platforms)
/// Modifications: support for custom name app (only for PulseAudio)
/// Modifications: completely removed pulseaudio in 1.3.0
use std::result;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dasp::Sample;
use rb::*;
use symphonia::core::audio::{AudioBufferRef, RawSample, SampleBuffer, SignalSpec};
use symphonia::core::conv::ConvertibleSample;
use symphonia::core::units::Duration;

pub trait AudioOutput {
    fn write(&mut self, decoded: AudioBufferRef<'_>, volume: f32) -> Result<()>;
    fn flush(&mut self);
}

#[allow(dead_code)]
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum AudioOutputError {
    OpenStreamError,
    PlayStreamError,
    StreamClosedError,
}

pub type Result<T> = result::Result<T, AudioOutputError>;

pub struct CpalAudioOutput;

trait AudioOutputSample: Sample + ConvertibleSample + RawSample + Send + 'static {}

impl AudioOutputSample for f32 {}

impl AudioOutputSample for i16 {}

impl AudioOutputSample for u16 {}

impl CpalAudioOutput {
    pub fn try_open(spec: SignalSpec, duration: Duration) -> Result<Box<dyn AudioOutput>> {
        // Get default host.
        let host = cpal::default_host();

        // Get the default audio output device.
        let device = match host.default_output_device() {
            Some(device) => device,
            _ => {
                eprintln!("Failed to get default audio output device");
                return Err(AudioOutputError::OpenStreamError);
            }
        };

        let config = match device.default_output_config() {
            Ok(config) => config,
            Err(err) => {
                eprintln!(
                    "Failed to get default audio output device config: {:?}",
                    err
                );
                return Err(AudioOutputError::OpenStreamError);
            }
        };

        // Select proper playback routine based on sample format.
        match config.sample_format() {
            cpal::SampleFormat::F32 => {
                CpalAudioOutputImpl::<f32>::try_open(spec, duration, &device)
            }
            cpal::SampleFormat::I16 => {
                CpalAudioOutputImpl::<i16>::try_open(spec, duration, &device)
            }
            cpal::SampleFormat::U16 => {
                CpalAudioOutputImpl::<u16>::try_open(spec, duration, &device)
            }
            _ => {
                unimplemented!("sample format not yet implemented")
            }
        }
    }
}

struct CpalAudioOutputImpl<T: AudioOutputSample>
where
    T: AudioOutputSample,
{
    ring_buf_producer: Producer<T>,
    sample_buf: SampleBuffer<T>,
    stream: cpal::Stream,
}

impl<T: AudioOutputSample + cpal::SizedSample> CpalAudioOutputImpl<T> {
    pub fn try_open(
        spec: SignalSpec,
        duration: Duration,
        device: &cpal::Device,
    ) -> Result<Box<dyn AudioOutput>> {
        let num_channels = spec.channels.count();

        // Output audio stream config.
        let config = cpal::StreamConfig {
            channels: num_channels as cpal::ChannelCount,
            sample_rate: cpal::SampleRate(spec.rate),
            buffer_size: cpal::BufferSize::Default,
        };

        // Create a ring buffer with a capacity for up-to 1000ms of audio.
        let ring_len = ((1000 * spec.rate as usize) / 1000) * num_channels;

        let ring_buf = SpscRb::new(ring_len);
        let (ring_buf_producer, ring_buf_consumer) = (ring_buf.producer(), ring_buf.consumer());

        let stream_result = device.build_output_stream(
            &config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                // Write out as many samples as possible from the ring buffer to the audio
                // output.
                let written = ring_buf_consumer.read(data).unwrap_or(0);
                // Mute any remaining samples.
                data[written..].iter_mut().for_each(|s| *s = T::MID);
            },
            move |err| eprintln!("audio output error: {:?}", err),
            None,
        );

        if let Err(err) = stream_result {
            eprintln!("audio output stream open error: {:?}", err);

            return Err(AudioOutputError::OpenStreamError);
        }

        let stream = stream_result.unwrap();

        // Start the output stream.
        if let Err(err) = stream.play() {
            eprintln!("audio output stream play error: {:?}", err);

            return Err(AudioOutputError::PlayStreamError);
        }

        let sample_buf = SampleBuffer::<T>::new(duration, spec);

        Ok(Box::new(CpalAudioOutputImpl {
            ring_buf_producer,
            sample_buf,
            stream,
        }))
    }
}

impl<T: AudioOutputSample> AudioOutput for CpalAudioOutputImpl<T> {
    fn write(&mut self, decoded: AudioBufferRef<'_>, volume: f32) -> Result<()> {
        // Do nothing if there are no audio frames.
        if decoded.frames() == 0 {
            return Ok(());
        }

        // Audio samples must be interleaved for cpal. Interleave the samples in the audio
        // buffer into the sample buffer.
        self.sample_buf.copy_interleaved_ref(decoded);

        // Write all the interleaved samples to the ring buffer.
        let mut samples: Vec<T> = self.sample_buf.samples().to_vec();
        for sample in samples.iter_mut() {
            *sample = sample.mul_amp(volume.to_sample());
        }

        while let Some(written) = self.ring_buf_producer.write_blocking(samples.as_slice()) {
            samples = samples[written..].to_vec();
        }

        Ok(())
    }

    fn flush(&mut self) {
        // Flush is best-effort, ignore the returned result.
        let _ = self.stream.pause();
    }
}

pub fn try_open(spec: SignalSpec, duration: Duration) -> Result<Box<dyn AudioOutput>> {
    CpalAudioOutput::try_open(spec, duration)
}
