//! The audio output: one long-lived stream to the default device, fed with f32 frames in the
//! device's own rate and channel layout. The stream callback applies the volume (ramped, so
//! changes, pauses and seeks do not click), soft-clips and converts to the device's sample type.
//!
//! Started from [symphonia-play's `output.rs`](https://github.com/pdeljanov/Symphonia/blob/master/symphonia-play/src/output.rs)
//! by [Philip Deljanov](https://github.com/pdeljanov).
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dasp::sample::FromSample;
use dasp::Sample;
use rtrb::{Consumer, Producer, RingBuffer};
use std::marker::PhantomData;
use std::result;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::Thread;
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
const STARTUP_GRACE: WallDuration = WallDuration::from_millis(250);
const STALL_TIMEOUT: WallDuration = WallDuration::from_secs(2);
pub const DEVICE_CHECK_INTERVAL: WallDuration = WallDuration::from_millis(500);

struct DefaultDeviceMonitor {
    changed: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
    thread: std::thread::Thread,
}

impl DefaultDeviceMonitor {
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd"
    ))]
    fn new(device_id: cpal::DeviceId) -> Result<Self> {
        let changed = Arc::new(AtomicBool::new(false));
        let stopped = Arc::new(AtomicBool::new(false));
        let changed_thread = changed.clone();
        let stopped_thread = stopped.clone();
        let worker = std::thread::Builder::new()
            .name("audio-device-monitor".into())
            .spawn(move || {
                let Ok(host) = cpal::host_from_id(device_id.host()) else {
                    changed_thread.store(true, Ordering::Relaxed);
                    return;
                };
                loop {
                    std::thread::park_timeout(DEVICE_CHECK_INTERVAL);
                    if stopped_thread.load(Ordering::Relaxed) {
                        break;
                    }
                    // PulseAudio enumeration waits for the server. Never do it on the
                    // playback worker, which must remain able to handle pause/seek/stop.
                    let changed = match host.default_output_device() {
                        Some(device) => device.id().is_ok_and(|id| id != device_id),
                        None => true,
                    };
                    if changed {
                        changed_thread.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            })
            .map_err(|error| {
                cpal::Error::with_message(cpal::ErrorKind::ResourceExhausted, error.to_string())
            })?;
        Ok(Self {
            changed,
            stopped,
            thread: worker.thread().clone(),
        })
    }

    fn changed(&self) -> bool {
        self.changed.load(Ordering::Relaxed)
    }
}

impl Drop for DefaultDeviceMonitor {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        self.thread.unpark();
        // Don't join a monitor that may still be waiting for an unresponsive server.
    }
}

struct BufferPolicy {
    target: usize,
    minimum: usize,
    maximum: usize,
    step: usize,
    strikes: usize,
    window: Instant,
    clean_since: Instant,
}

impl BufferPolicy {
    fn new(rate: u32, now: Instant) -> Self {
        let minimum = (rate as usize / 20).max(1);
        Self {
            target: minimum,
            minimum,
            maximum: (rate as usize / 2).max(1),
            step: (rate as usize / 40).max(1),
            strikes: 0,
            window: now,
            clean_since: now,
        }
    }

    fn reset(&mut self, now: Instant) {
        self.strikes = 0;
        self.window = now;
        self.clean_since = now;
    }

    fn update(&mut self, now: Instant, starvations: usize, request: usize) {
        // Two callback periods leave room for scheduling jitter. Never resize the ring.
        let floor = self
            .minimum
            .max(request.saturating_mul(2))
            .min(self.maximum);

        self.target = self.target.max(floor);
        if now.duration_since(self.window) >= WallDuration::from_secs(2) {
            self.strikes = 0;
            self.window = now;
        }

        if starvations > 0 {
            self.clean_since = now;
            self.strikes = self.strikes.saturating_add(starvations);

            if self.strikes >= 3 {
                self.target = (self.target + self.step).min(self.maximum);
                self.strikes = 0;
                self.window = now;
            }
        } else if now.duration_since(self.clean_since) >= WallDuration::from_secs(10) {
            self.target = self
                .target
                .saturating_sub((self.step / 5).max(1))
                .max(floor);
            self.clean_since = now;
        }
    }
}

/// State shared between the writer and the stream callback.
struct SharedState {
    origin: Instant,
    consumed: AtomicU64,
    discard_until: AtomicU64,
    device_until_ns: AtomicU64,
    heartbeat_ns: AtomicU64,
    request_frames: AtomicUsize,
    starvations: AtomicUsize,
    monitoring: AtomicBool,
    failed: AtomicBool,
    /// Target gain, as `f32` bits.
    volume: AtomicU32,
    /// Fade to silence instead of `volume`; queued audio is kept, not played.
    muted: AtomicBool,
    /// Set by the callback while muted and fully faded out.
    silent: AtomicBool,
    /// The callback wakes the writer once fewer samples than this are queued; 0 when the
    /// writer is not waiting.
    wake_below: AtomicUsize,
    writer: Mutex<Option<Thread>>,
}

impl SharedState {
    fn new(volume: f32) -> Self {
        Self {
            origin: Instant::now(),
            consumed: AtomicU64::new(0),
            discard_until: AtomicU64::new(0),
            device_until_ns: AtomicU64::new(0),
            heartbeat_ns: AtomicU64::new(0),
            request_frames: AtomicUsize::new(0),
            starvations: AtomicUsize::new(0),
            monitoring: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            volume: AtomicU32::new(volume.to_bits()),
            muted: AtomicBool::new(false),
            silent: AtomicBool::new(false),
            wake_below: AtomicUsize::new(0),
            writer: Mutex::new(None),
        }
    }

    fn now_ns(&self) -> u64 {
        self.origin.elapsed().as_nanos() as u64
    }

    fn stream_error(&self, kind: cpal::ErrorKind) {
        match kind {
            // CPAL already rerouted the stream, or only failed to boost its priority.
            cpal::ErrorKind::DeviceChanged | cpal::ErrorKind::RealtimeDenied => {}
            cpal::ErrorKind::Xrun => {
                self.starvations.fetch_add(1, Ordering::Relaxed);
            }
            _ => self.failed.store(true, Ordering::Relaxed),
        }
        self.wake();
    }

    /// Unparks the writer, unless it is being replaced right now (it then has a timeout).
    fn wake(&self) {
        if let Ok(writer) = self.writer.try_lock() {
            if let Some(writer) = writer.as_ref() {
                writer.unpark();
            }
        }
    }

    fn pending_frames(&self, produced: u64, channels: usize, rate: u32) -> usize {
        // A concurrent callback can cause a brief overestimate, never an early EOF.
        let consumed = self.consumed.load(Ordering::Acquire);
        let discarded = self.discard_until.load(Ordering::Relaxed);
        let queued = produced.saturating_sub(consumed.max(discarded)) as usize;
        let device_ns = self
            .device_until_ns
            .load(Ordering::Relaxed)
            .saturating_sub(self.now_ns());
        queued / channels + (device_ns as f64 * rate as f64 / 1_000_000_000.0).ceil() as usize
    }
}

trait OutputSample: cpal::SizedSample + Sample + FromSample<f32> + Send + 'static {
    /// One least significant bit, the dither amplitude; 0 when the format needs no dither.
    const LSB: f32;
}

impl OutputSample for f32 {
    const LSB: f32 = 0.0;
}

impl OutputSample for i32 {
    const LSB: f32 = 0.0;
}

impl OutputSample for i16 {
    const LSB: f32 = 1.0 / 32768.0;
}

impl OutputSample for u16 {
    const LSB: f32 = 1.0 / 32768.0;
}

struct Callback<T> {
    consumer: Consumer<f32>,
    consumed: u64,
    state: Arc<SharedState>,
    channels: usize,
    rate: u32,
    mix: Vec<f32>,
    gain: f32,
    /// Gain change per frame while ramping.
    step: f32,
    clip_mem: Vec<f32>,
    noise: u32,
    sample: PhantomData<T>,
}

impl<T: OutputSample> Callback<T> {
    fn render(&mut self, data: &mut [T], playback_delay: WallDuration) {
        let state = &*self.state;
        let now = state.now_ns();
        state.heartbeat_ns.store(now, Ordering::Relaxed);
        state
            .request_frames
            .fetch_max(data.len().div_ceil(self.channels), Ordering::Relaxed);
        // Only the consumer advances the read index, including on seek while paused.
        let discard = state.discard_until.load(Ordering::Acquire);
        let skip = discard.saturating_sub(self.consumed) as usize;
        if skip > 0 {
            if let Ok(chunk) = self.consumer.read_chunk(skip) {
                chunk.commit_all();
                self.consumed += skip as u64;
                self.clip_mem.fill(0.0);
            }
        }

        let requested = data.len() / self.channels * self.channels;
        if self.mix.len() < requested {
            self.mix.resize(requested, 0.0);
        }
        let mix = &mut self.mix[..requested];
        let muted = state.muted.load(Ordering::Relaxed);
        let target = if muted {
            0.0
        } else {
            f32::from_bits(state.volume.load(Ordering::Relaxed))
        };
        // Faded out: keep the queue for when the fade-in starts.
        let written = if muted && self.gain == 0.0 {
            0
        } else {
            self.consumer.pop_partial_slice(mix).0.len()
        };
        mix[written..].fill(0.0);
        self.consumed += written as u64;
        if written > 0 {
            let duration = WallDuration::from_secs_f64(
                written as f64 / self.channels as f64 / self.rate as f64,
            );
            state.device_until_ns.store(
                now.saturating_add(playback_delay.as_nanos() as u64)
                    .saturating_add(duration.as_nanos() as u64),
                Ordering::Relaxed,
            );
        }
        // Publish the device deadline before removing these samples from pending_frames.
        state.consumed.store(self.consumed, Ordering::Release);
        if written < requested && !muted && state.monitoring.load(Ordering::Relaxed) {
            state.starvations.fetch_add(1, Ordering::Relaxed);
        }

        for frame in mix.chunks_exact_mut(self.channels) {
            if self.gain != target {
                self.gain = if self.gain < target {
                    (self.gain + self.step).min(target)
                } else {
                    (self.gain - self.step).max(target)
                };
            }
            for sample in frame {
                *sample *= self.gain;
            }
        }
        state
            .silent
            .store(muted && self.gain == 0.0, Ordering::Release);
        soft_clip(mix, self.channels, &mut self.clip_mem);
        for (out, &sample) in data.iter_mut().zip(mix.iter()) {
            *out = T::from_sample(dither::<T>(sample, &mut self.noise));
        }
        data[requested..].fill(T::EQUILIBRIUM);

        let wake_below = state.wake_below.load(Ordering::Acquire);
        if wake_below > 0
            && self.consumer.slots() < wake_below
            && state.wake_below.swap(0, Ordering::AcqRel) != 0
        {
            state.wake();
        }
    }
}

/// TPDF dither of one LSB, for integer formats; silence stays silent.
fn dither<T: OutputSample>(sample: f32, noise: &mut u32) -> f32 {
    if T::LSB == 0.0 || sample == 0.0 {
        return sample;
    }
    let mut uniform = || {
        // xorshift32
        *noise ^= *noise << 13;
        *noise ^= *noise >> 17;
        *noise ^= *noise << 5;
        (*noise >> 8) as f32 / (1 << 24) as f32
    };
    sample + (uniform() - uniform()) * T::LSB
}

/// The stream to the default output device. Writing never blocks: callers wait with
/// [`Output::wait_for_room`], which the stream callback wakes up.
pub struct Output {
    format: OutputFormat,
    producer: Producer<f32>,
    stream: cpal::Stream,
    state: Arc<SharedState>,
    /// Samples ever written.
    produced: u64,
    policy: BufferPolicy,
    monitor_after: Option<Instant>,
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
        let requested_frames = (format.rate / 100).max(1);
        let buffer_size = match config.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => {
                cpal::BufferSize::Fixed(requested_frames.clamp(*min, *max))
            }
            cpal::SupportedBufferSize::Unknown => cpal::BufferSize::Default,
        };
        let stream_config = cpal::StreamConfig {
            channels: config.channels(),
            sample_rate: config.sample_rate(),
            buffer_size,
        };
        let policy = BufferPolicy::new(format.rate, Instant::now());
        let (producer, consumer) = RingBuffer::new(policy.maximum * format.channels);
        let state = Arc::new(SharedState::new(volume));
        let mut callback = Callback::<T> {
            consumer,
            consumed: 0,
            state: state.clone(),
            channels: format.channels,
            rate: format.rate,
            mix: vec![0.0; requested_frames as usize * format.channels],
            gain: 0.0,
            step: 1.0 / (RAMP.as_secs_f32() * format.rate as f32).max(1.0),
            clip_mem: vec![0.0; format.channels],
            noise: 0x9E37_79B9,
            sample: PhantomData,
        };
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
            policy,
            monitor_after: None,
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
        self.policy.maximum * self.format.channels - self.producer.slots()
    }

    /// Queues as much of `samples` (whole frames) as the buffer target allows and returns how
    /// many samples were taken.
    pub fn write(&mut self, samples: &[f32]) -> Result<usize> {
        self.check_health()?;
        if self.paused {
            return Ok(0);
        }
        let now = Instant::now();
        let monitor_after = *self.monitor_after.get_or_insert(now + STARTUP_GRACE);
        let monitoring = now >= monitor_after;
        self.state.monitoring.store(monitoring, Ordering::Relaxed);
        let starvations = self.state.starvations.swap(0, Ordering::Relaxed);
        if !monitoring {
            self.policy.reset(now);
        }
        self.policy.update(
            now,
            if monitoring { starvations } else { 0 },
            self.state.request_frames.load(Ordering::Relaxed),
        );
        // Backpressure only above the target: successive writes fill toward it. A lower
        // target drains naturally; it never discards playable samples.
        let room = (self.policy.target * self.format.channels).saturating_sub(self.queued());
        let count = room.min(samples.len()) / self.format.channels * self.format.channels;
        let (written, _) = self.producer.push_partial_slice(&samples[..count]);
        self.produced += written.len() as u64;
        Ok(written.len())
    }

    /// Parks the attached thread until the queue drains to half the target, or `timeout`.
    /// Anyone holding the thread handle may also unpark it earlier.
    pub fn wait_for_room(&self, timeout: WallDuration) {
        let low = self.policy.target * self.format.channels / 2;
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
        self.state.monitoring.store(false, Ordering::Relaxed);
        self.state
            .discard_until
            .store(self.produced, Ordering::Release);
        self.state.starvations.store(0, Ordering::Relaxed);
        self.monitor_after = None;
        self.policy.reset(Instant::now());
        if !self.paused {
            self.state.muted.store(false, Ordering::Relaxed);
        }
    }

    /// Pausing fades out first; resuming fades back in.
    pub fn set_paused(&mut self, paused: bool) -> Result<()> {
        if paused == self.paused {
            return Ok(());
        }
        self.state.monitoring.store(false, Ordering::Relaxed);
        self.state.starvations.store(0, Ordering::Relaxed);
        self.monitor_after = None;
        self.policy.reset(Instant::now());
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

    /// The queue runs dry on purpose from now on; that is not starvation.
    pub fn set_draining(&mut self) {
        self.state.monitoring.store(false, Ordering::Relaxed);
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

/// Port of `opus_pcm_soft_clip` from libopus, Copyright (c) 2011 Xiph.Org Foundation, Skype Limited (BSD-3-Clause).
fn soft_clip(samples: &mut [f32], channels: usize, mem: &mut [f32]) {
    if channels == 0 || samples.len() < channels {
        return;
    }
    let frames = samples.len() / channels;

    for sample in samples.iter_mut() {
        *sample = sample.clamp(-2.0, 2.0);
    }

    for (channel, declip) in mem.iter_mut().enumerate().take(channels) {
        let x = |i: usize| i * channels + channel;
        let mut a = *declip;

        for i in 0..frames {
            let v = samples[x(i)];
            if v * a >= 0.0 {
                break;
            }
            samples[x(i)] = v + a * v * v;
        }

        let mut curr = 0;
        let x0 = samples[x(0)];
        loop {
            let Some(i) = (curr..frames).find(|&i| samples[x(i)].abs() > 1.0) else {
                a = 0.0;
                break;
            };

            let pivot = samples[x(i)];
            let mut peak_pos = i;
            let mut start = i;
            let mut end = i;
            let mut maxval = pivot.abs();
            while start > 0 && pivot * samples[x(start - 1)] >= 0.0 {
                start -= 1;
            }
            while end < frames && pivot * samples[x(end)] >= 0.0 {
                if samples[x(end)].abs() > maxval {
                    maxval = samples[x(end)].abs();
                    peak_pos = end;
                }
                end += 1;
            }
            let special = start == 0 && pivot * samples[x(0)] >= 0.0;

            a = (maxval - 1.0) / (maxval * maxval);
            a += a * 2.4e-7;
            if pivot > 0.0 {
                a = -a;
            }
            for j in start..end {
                let v = samples[x(j)];
                samples[x(j)] = v + a * v * v;
            }

            if special && peak_pos >= 2 {
                let mut offset = x0 - samples[x(0)];
                let delta = offset / peak_pos as f32;
                for j in curr..peak_pos {
                    offset -= delta;
                    samples[x(j)] = (samples[x(j)] + offset).clamp(-1.0, 1.0);
                }
            }

            curr = end;
            if curr == frames {
                break;
            }
        }
        *declip = a;
    }
}
