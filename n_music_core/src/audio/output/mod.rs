//! The audio output: one long-lived stream to the default device, fed with f32 frames in the
//! device's own rate and channel layout. The stream callback applies the volume (ramped, so
//! changes, pauses and seeks do not click), soft-clips and converts to the device's sample type.
//!
//! The writer keeps the queue full. Nothing waits for queued audio to play out: volume and
//! pausing act in the callback and seeking discards the queue, so its size only buys
//! robustness against stalls and fewer wakeups.
//!
//! Started from [symphonia-play's `output.rs`](https://github.com/pdeljanov/Symphonia/blob/master/symphonia-play/src/output.rs)
//! by [Philip Deljanov](https://github.com/pdeljanov).

mod callback;
mod device_monitor;
mod dsp;

use callback::{Callback, OutputSample, SharedState};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use device_monitor::DefaultDeviceMonitor;
use rtrb::{Producer, RingBuffer};
use std::result;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration as WallDuration, Instant};

/// What the device plays; every track is converted to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputFormat {
    pub rate: u32,
    pub channels: usize,
}

#[derive(Debug)]
pub enum AudioOutputError {
    StreamClosedError,
    Backend(cpal::Error),
}

pub type Result<T> = result::Result<T, AudioOutputError>;

impl std::fmt::Display for AudioOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StreamClosedError => f.write_str("Audio output stream closed or stalled"),
            Self::Backend(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for AudioOutputError {}

impl From<cpal::Error> for AudioOutputError {
    fn from(error: cpal::Error) -> Self {
        Self::Backend(error)
    }
}

impl AudioOutputError {
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::StreamClosedError => true,
            Self::Backend(error) => matches!(
                error.kind(),
                cpal::ErrorKind::DeviceBusy
                    | cpal::ErrorKind::DeviceNotAvailable
                    | cpal::ErrorKind::HostUnavailable
                    | cpal::ErrorKind::StreamInvalidated
                    | cpal::ErrorKind::BackendError
            ),
        }
    }
}

/// Volume changes and fades take this long from silence to full scale.
const RAMP: WallDuration = WallDuration::from_millis(10);
/// Longest wait for a fade-out, in case the callback stopped coming.
const FADE_TIMEOUT: WallDuration = WallDuration::from_millis(100);
const STALL_TIMEOUT: WallDuration = WallDuration::from_secs(2);
pub const DEVICE_CHECK_INTERVAL: WallDuration = WallDuration::from_millis(500);
/// Audio queued ahead of the device. The writer refills it once half is played.
const QUEUE: WallDuration = WallDuration::from_millis(500);
/// The device's period: what pause and seek fades wait for, as queued audio is not.
const PERIOD: WallDuration = WallDuration::from_millis(10);

/// The stream to the default output device. Writing never blocks: callers wait with
/// [`Output::wait_for_room`], which the stream callback wakes up.
pub struct Output {
    format: OutputFormat,
    producer: Producer<f32>,
    stream: cpal::Stream,
    state: Arc<SharedState>,
    /// Samples ever written.
    produced: u64,
    /// The queue's size in samples.
    capacity: usize,
    paused: bool,
    default_device: Option<DefaultDeviceMonitor>,
}

impl Output {
    /// Opens the default device in its default configuration, playing at `volume`.
    pub fn open(volume: f32) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| cpal::Error::new(cpal::ErrorKind::DeviceNotAvailable))?;
        let config = device.default_output_config()?;
        let default_device = match host.id() {
            // CPAL's PulseAudio backend targets a concrete sink and doesn't watch defaults.
            // WASAPI/CoreAudio supply native notifications; AAudio also has the Android bridge.
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd"
            ))]
            cpal::HostId::PulseAudio => Some(DefaultDeviceMonitor::new(device.id()?)?),
            _ => None,
        };

        match config.sample_format() {
            cpal::SampleFormat::F32 => Self::build::<f32>(&device, &config, default_device, volume),
            cpal::SampleFormat::I32 => Self::build::<i32>(&device, &config, default_device, volume),
            cpal::SampleFormat::I16 => Self::build::<i16>(&device, &config, default_device, volume),
            cpal::SampleFormat::U16 => Self::build::<u16>(&device, &config, default_device, volume),
            format => Err(cpal::Error::with_message(
                cpal::ErrorKind::UnsupportedConfig,
                format!("Unsupported output sample format: {format}"),
            )
            .into()),
        }
    }

    fn build<T: OutputSample>(
        device: &cpal::Device,
        config: &cpal::SupportedStreamConfig,
        default_device: Option<DefaultDeviceMonitor>,
        volume: f32,
    ) -> Result<Self> {
        let format = OutputFormat {
            rate: config.sample_rate(),
            channels: usize::from(config.channels()),
        };
        let period = ((format.rate as f64 * PERIOD.as_secs_f64()) as u32).max(1);
        let buffer_size = match config.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => {
                cpal::BufferSize::Fixed(period.clamp(*min, *max))
            }
            cpal::SupportedBufferSize::Unknown => cpal::BufferSize::Default,
        };
        let frames = ((format.rate as f64 * QUEUE.as_secs_f64()) as usize).max(1);
        let stream_config = cpal::StreamConfig {
            channels: config.channels(),
            sample_rate: config.sample_rate(),
            buffer_size,
        };
        let capacity = frames * format.channels;
        let (producer, consumer) = RingBuffer::new(capacity);
        let state = Arc::new(SharedState::new(volume));
        // A callback never gets more than the whole queue, so mixing never allocates.
        let mut callback = Callback::<T>::new(consumer, state.clone(), format, frames);
        let error_state = state.clone();
        let stream = device.build_output_stream(
            stream_config,
            move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
                let timestamp = info.timestamp();
                callback.render(data, timestamp.playback.duration_since(timestamp.callback));
            },
            move |err| error_state.stream_error(err.kind()),
            Some(STALL_TIMEOUT),
        )?;
        stream.play()?;
        log::debug!(
            "Opened audio output: {} Hz, {} channels, {:?}",
            format.rate,
            format.channels,
            config.sample_format()
        );

        Ok(Self {
            format,
            producer,
            stream,
            state,
            produced: 0,
            capacity,
            paused: false,
            default_device,
        })
    }

    pub fn format(&self) -> OutputFormat {
        self.format
    }

    /// Makes the calling thread the one [`Self::wait_for_room`] parks and the callback wakes.
    pub fn attach(&self) {
        *self.state.writer.lock().unwrap() = Some(std::thread::current());
    }

    pub fn set_volume(&self, volume: f32) {
        self.state.volume.store(volume.to_bits(), Ordering::Relaxed);
    }

    fn queued(&self) -> usize {
        self.capacity - self.producer.slots()
    }

    /// Queues as much of `samples` (whole frames) as fits and returns how many samples were
    /// taken.
    pub fn write(&mut self, samples: &[f32]) -> Result<usize> {
        self.check_health()?;
        if self.paused {
            return Ok(0);
        }
        let room = self.producer.slots().min(samples.len());
        let count = room / self.format.channels * self.format.channels;
        let (written, _) = self.producer.push_partial_slice(&samples[..count]);
        self.produced += written.len() as u64;
        Ok(written.len())
    }

    /// Parks the attached thread until half the queue is free, or `timeout`. Anyone holding
    /// the thread handle may also unpark it earlier.
    pub fn wait_for_room(&self, timeout: WallDuration) {
        let low = self.capacity / 2;
        self.park(low + 1, || self.queued() <= low, timeout);
    }

    /// Parks the attached thread until everything queued was handed to the device, or
    /// `timeout`.
    pub fn wait_drained(&self, timeout: WallDuration) {
        self.park(1, || self.queued() == 0, timeout);
    }

    fn park(&self, wake_below: usize, done: impl Fn() -> bool, timeout: WallDuration) {
        self.state.wake_below.store(wake_below, Ordering::Release);
        // A wake-up between the check and the park leaves the token, so park returns at once.
        if !done() {
            std::thread::park_timeout(timeout);
        }
        self.state.wake_below.store(0, Ordering::Release);
    }

    /// Fades out, waiting for the callback to get there.
    fn fade_out(&self) {
        self.state.muted.store(true, Ordering::Relaxed);
        if self.paused {
            return;
        }
        let deadline = Instant::now() + FADE_TIMEOUT;
        loop {
            let now = Instant::now();
            if self.state.silent.load(Ordering::Acquire)
                || now >= deadline
                || self.check_health().is_err()
            {
                break;
            }
            // Wake on the next callback.
            self.park(
                usize::MAX,
                || self.state.silent.load(Ordering::Acquire),
                deadline - now,
            );
        }
    }

    /// Frames ever written.
    pub fn produced_frames(&self) -> u64 {
        self.produced / self.format.channels as u64
    }

    /// Frames written but not heard yet, including the device's own buffer.
    pub fn pending_frames(&self) -> usize {
        self.state
            .pending_frames(self.produced, self.format.channels, self.format.rate)
    }

    /// Frames heard so far, estimated.
    pub fn played_frames(&self) -> u64 {
        self.produced_frames()
            .saturating_sub(self.pending_frames() as u64)
    }

    /// Fades out, then drops everything queued.
    pub fn discard_queued(&mut self) {
        self.fade_out();
        self.state
            .discard_until
            .store(self.produced, Ordering::Release);
        if !self.paused {
            self.state.muted.store(false, Ordering::Relaxed);
        }
    }

    /// Pausing fades out first; resuming fades back in.
    pub fn set_paused(&mut self, paused: bool) -> Result<()> {
        if paused == self.paused {
            return Ok(());
        }
        self.state
            .heartbeat_ns
            .store(self.state.now_ns(), Ordering::Relaxed);
        if paused {
            self.fade_out();
            self.stream.pause()?;
        } else {
            self.state.muted.store(false, Ordering::Relaxed);
            self.stream.play()?;
        }
        self.paused = paused;
        Ok(())
    }

    pub fn check_health(&self) -> Result<()> {
        if self.state.failed.load(Ordering::Relaxed)
            || self.producer.is_abandoned()
            || self
                .default_device
                .as_ref()
                .is_some_and(DefaultDeviceMonitor::changed)
            || (!self.paused
                && self
                    .state
                    .now_ns()
                    .saturating_sub(self.state.heartbeat_ns.load(Ordering::Relaxed))
                    > STALL_TIMEOUT.as_nanos() as u64)
        {
            Err(AudioOutputError::StreamClosedError)
        } else {
            Ok(())
        }
    }
}
