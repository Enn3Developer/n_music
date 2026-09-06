//! Platform-dependant Audio Outputs

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleRate;
use dasp::Sample;
use rb::*;
/// This is a modified version of [symphonia-play's `output.rs`](https://github.com/pdeljanov/Symphonia/blob/master/symphonia-play/src/output.rs)
/// It was originally made by [Philip Deljanov](https://github.com/pdeljanov)
/// Modifications: support for volume (for all platforms)
/// Modifications: support for custom name app (only for PulseAudio)
/// Modifications: completely removed pulseaudio in 1.3.0
use std::result;
use std::sync::{Arc, Mutex};
use std::time::{Duration as WallDuration, Instant};
use symphonia::core::audio::{AudioBufferRef, RawSample, SampleBuffer, SignalSpec};
use symphonia::core::conv::ConvertibleSample;
use symphonia::core::units::Duration;

pub trait AudioOutput {
    fn write(&mut self, decoded: AudioBufferRef<'_>, volume: f32, skip_frames: usize)
        -> Result<()>;
    fn pending_frames(&self) -> usize;
    fn discard_queued(&mut self);
    fn set_paused(&mut self, paused: bool) -> Result<()>;
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

impl std::fmt::Display for AudioOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for AudioOutputError {}

pub struct CpalAudioOutput;

trait AudioOutputSample: Sample + ConvertibleSample + RawSample + Send + 'static {}

impl AudioOutputSample for f32 {}

impl AudioOutputSample for i32 {}

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
            cpal::SampleFormat::I32 => {
                CpalAudioOutputImpl::<i32>::try_open(spec, duration, &device)
            }
            cpal::SampleFormat::I16 => {
                CpalAudioOutputImpl::<i16>::try_open(spec, duration, &device)
            }
            cpal::SampleFormat::U16 => {
                CpalAudioOutputImpl::<u16>::try_open(spec, duration, &device)
            }
            _ => {
                unimplemented!(
                    "sample format not yet implemented: {}",
                    config.sample_format()
                )
            }
        }
    }
}

struct CpalAudioOutputImpl<T: AudioOutputSample>
where
    T: AudioOutputSample,
{
    ring_buf: SpscRb<T>,
    channels: usize,
    sample_rate: u32,
    ring_buf_producer: Producer<T>,
    sample_buf: SampleBuffer<T>,
    stream: cpal::Stream,
    queue_access: Arc<Mutex<Instant>>,
}

impl<T: AudioOutputSample + cpal::SizedSample> CpalAudioOutputImpl<T> {
    pub fn try_open(
        spec: SignalSpec,
        duration: Duration,
        device: &cpal::Device,
    ) -> Result<Box<dyn AudioOutput>> {
        let num_channels = spec.channels.count();

        let requested_frames = (spec.rate / 100).max(1);
        let buffer_size = device
            .supported_output_configs()
            .ok()
            .and_then(|configs| {
                configs
                    .filter(|config| {
                        config.channels() as usize == num_channels
                            && config.sample_format() == <T as cpal::SizedSample>::FORMAT
                            && config.min_sample_rate() <= spec.rate
                            && config.max_sample_rate() >= spec.rate
                    })
                    .find_map(|config| match config.buffer_size() {
                        cpal::SupportedBufferSize::Range { min, max } => {
                            Some(cpal::BufferSize::Fixed(requested_frames.clamp(*min, *max)))
                        }
                        cpal::SupportedBufferSize::Unknown => None,
                    })
            })
            .unwrap_or(cpal::BufferSize::Default);
        let config = cpal::StreamConfig {
            channels: num_channels as cpal::ChannelCount,
            sample_rate: SampleRate::from(spec.rate),
            buffer_size,
        };
        let ring_len = (spec.rate as usize / 20).max(1) * num_channels;

        let ring_buf = SpscRb::new(ring_len);
        let (ring_buf_producer, ring_buf_consumer) = (ring_buf.producer(), ring_buf.consumer());

        let queue_access = Arc::new(Mutex::new(Instant::now()));
        let callback_queue_access = queue_access.clone();
        let stream_result = device.build_output_stream(
            config,
            move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
                // Write out as many samples as possible from the ring buffer to the audio
                // output.
                let written = {
                    let mut device_buffered_until = callback_queue_access.lock().unwrap();
                    let written = ring_buf_consumer.read(data).unwrap_or(0);
                    let timestamp = info.timestamp();
                    if written > 0 {
                        *device_buffered_until = Instant::now()
                            + timestamp.playback.duration_since(timestamp.callback)
                            + WallDuration::from_secs_f64(
                                written as f64 / num_channels as f64 / spec.rate as f64,
                            );
                    }
                    written
                };
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
            ring_buf,
            channels: num_channels,
            sample_rate: spec.rate,
            ring_buf_producer,
            sample_buf,
            stream,
            queue_access,
        }))
    }
}

impl<T: AudioOutputSample> AudioOutput for CpalAudioOutputImpl<T> {
    fn write(
        &mut self,
        decoded: AudioBufferRef<'_>,
        volume: f32,
        skip_frames: usize,
    ) -> Result<()> {
        // Do nothing if there are no audio frames.
        if decoded.frames() == 0 {
            return Ok(());
        }

        // Audio samples must be interleaved for cpal. Interleave the samples in the audio
        // buffer into the sample buffer.
        self.sample_buf.copy_interleaved_ref(decoded);

        // Write all the interleaved samples to the ring buffer.
        let samples = &mut self.sample_buf.samples_mut()[skip_frames * self.channels..];
        for sample in samples.iter_mut() {
            *sample = sample.mul_amp(volume.to_sample());
        }

        let mut remaining = &samples[..];
        while let Some(written) = self.ring_buf_producer.write_blocking(remaining) {
            remaining = &remaining[written..];
        }

        Ok(())
    }

    fn pending_frames(&self) -> usize {
        let device_buffered_until = self.queue_access.lock().unwrap();
        let device_frames = device_buffered_until
            .saturating_duration_since(Instant::now())
            .as_secs_f64()
            * self.sample_rate as f64;
        self.ring_buf.count() / self.channels + device_frames.ceil() as usize
    }

    fn set_paused(&mut self, paused: bool) -> Result<()> {
        if paused {
            self.stream.pause()
        } else {
            self.stream.play()
        }
        .map_err(|_| AudioOutputError::PlayStreamError)
    }

    fn discard_queued(&mut self) {
        let _guard = self.queue_access.lock().unwrap();
        self.ring_buf.clear();
    }
}

pub fn try_open(spec: SignalSpec, duration: Duration) -> Result<Box<dyn AudioOutput>> {
    CpalAudioOutput::try_open(spec, duration)
}
