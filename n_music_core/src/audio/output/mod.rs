//! The audio output: one long-lived stream to the chosen or the default device, fed with f32
//! frames in the device's own rate and channel layout. The stream callback applies the volume
//! (ramped, so changes, pauses and seeks do not click), soft-clips and converts to the device's
//! sample type.
//!
//! The writer keeps the queue full. Nothing waits for queued audio to play out: volume and
//! pausing act in the callback and seeking discards the queue, so its size only buys
//! robustness against stalls and fewer wakeups.
//!
//! The device's own buffer is another matter. A paused stream keeps what the device was handed
//! and has not played, and plays it once resumed, whatever track comes next. PulseAudio,
//! WASAPI, ALSA and AAudio all keep it. So the stream pauses only once the device has played
//! the fade-out.
//!
//! Started from [symphonia-play's `output.rs`](https://github.com/pdeljanov/Symphonia/blob/master/symphonia-play/src/output.rs)
//! by [Philip Deljanov](https://github.com/pdeljanov).

mod callback;
mod device_monitor;
mod dsp;

use crate::settings::OutputDevice;
use callback::{Callback, OutputSample, SharedState};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use device_monitor::DeviceMonitor;
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
/// Longest wait for the device to play a fade-out before the stream pauses, long enough for
/// Bluetooth. A device further behind plays what is left once resumed.
const PLAY_OUT_TIMEOUT: WallDuration = WallDuration::from_millis(500);
const STALL_TIMEOUT: WallDuration = WallDuration::from_secs(2);
pub const DEVICE_CHECK_INTERVAL: WallDuration = WallDuration::from_millis(500);
/// Audio queued ahead of the device. The writer refills it once half is played.
const QUEUE: WallDuration = WallDuration::from_millis(500);
/// The device's period: what pause and seek fades wait for, as queued audio is not.
const PERIOD: WallDuration = WallDuration::from_millis(10);

/// The devices the output can play on, as the system lists them.
pub(crate) fn output_devices() -> Vec<OutputDevice> {
    let devices = match cpal::default_host().output_devices() {
        Ok(devices) => devices,
        Err(error) => {
            log::warn!("Could not list the output devices: {error}");
            return vec![];
        }
    };
    devices
        .filter_map(|device| {
            let id = device.id().ok()?;
            let name = device
                .description()
                .map_or_else(|_| id.id().to_string(), |about| about.name().to_string());
            Some(OutputDevice {
                id: id.to_string(),
                name,
            })
        })
        .collect()
}

/// The device to play on: `chosen` while it is there, else the default.
fn device(host: &cpal::Host, chosen: Option<&cpal::DeviceId>) -> Option<cpal::Device> {
    chosen
        .and_then(|id| host.device_by_id(id))
        .or_else(|| host.default_output_device())
}

/// The stream to the output device. Writing never blocks: callers wait with
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
    monitor: Option<DeviceMonitor>,
}

impl Output {
    /// Opens `chosen`, an [`OutputDevice::id`], while it is there, else the default device, in
    /// its default configuration, playing at `volume`.
    pub fn open(volume: f32, chosen: Option<&str>) -> Result<Self> {
        let host = cpal::default_host();
        let chosen = chosen.and_then(|id| {
            id.parse::<cpal::DeviceId>()
                .inspect_err(|error| log::warn!("Ignoring the output device {id:?}: {error}"))
                .ok()
        });
        let device = self::device(&host, chosen.as_ref())
            .ok_or_else(|| cpal::Error::new(cpal::ErrorKind::DeviceNotAvailable))?;
        if let Some(chosen) = &chosen {
            if device.id().ok().as_ref() != Some(chosen) {
                log::info!("Output device {chosen} is not there, playing on the default");
            }
        }
        let config = device.default_output_config()?;
        let watch = match host.id() {
            // CPAL's PulseAudio backend targets a concrete sink and doesn't watch defaults.
            // WASAPI/CoreAudio supply native notifications; AAudio also has the Android bridge.
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd"
            ))]
            cpal::HostId::PulseAudio => true,
            // A chosen device is watched everywhere, to get back to it.
            _ => chosen.is_some(),
        };
        let monitor = if watch {
            Some(DeviceMonitor::new(device.id()?, chosen)?)
        } else {
            None
        };

        match config.sample_format() {
            cpal::SampleFormat::F32 => Self::build::<f32>(&device, &config, monitor, volume),
            cpal::SampleFormat::I32 => Self::build::<i32>(&device, &config, monitor, volume),
            cpal::SampleFormat::I16 => Self::build::<i16>(&device, &config, monitor, volume),
            cpal::SampleFormat::U16 => Self::build::<u16>(&device, &config, monitor, volume),
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
        monitor: Option<DeviceMonitor>,
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
            monitor,
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

    /// Waits for the device to play what the callback handed it, the fade-out last, so a
    /// paused stream keeps only silence.
    fn play_out(&self) {
        let left = self.state.device_left().min(PLAY_OUT_TIMEOUT);
        if !left.is_zero() {
            std::thread::sleep(left);
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

    /// Pausing fades out first and pauses the stream once the fade-out was heard; resuming
    /// fades back in.
    pub fn set_paused(&mut self, paused: bool) -> Result<()> {
        if paused == self.paused {
            return Ok(());
        }
        self.state
            .heartbeat_ns
            .store(self.state.now_ns(), Ordering::Relaxed);
        if paused {
            self.fade_out();
            self.play_out();
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
            || self.monitor.as_ref().is_some_and(DeviceMonitor::changed)
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
