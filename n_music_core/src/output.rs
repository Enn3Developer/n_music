//! Platform-dependant Audio Outputs

/// This is a modified version of [symphonia-play's `output.rs`](https://github.com/pdeljanov/Symphonia/blob/master/symphonia-play/src/output.rs)
/// It was originally made by [Philip Deljanov](https://github.com/pdeljanov)
/// Modifications: support for volume (for all platforms)
/// Modifications: support for custom name app (only for PulseAudio)
/// Modifications: completely removed pulseaudio in 1.3.0
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleRate;
use dasp::Sample;
use rtrb::{Consumer, Producer, RingBuffer};
use std::result;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration as WallDuration, Instant};
use symphonia::core::audio::{
    conv::{ConvertibleSample, FromSample},
    AudioSpec, GenericAudioBufferRef,
};

pub trait AudioOutput {
    fn write(
        &mut self,
        decoded: GenericAudioBufferRef<'_>,
        volume: f32,
        skip_frames: usize,
    ) -> Result<()>;
    fn pending_frames(&self) -> usize;
    fn discard_queued(&mut self);
    fn set_paused(&mut self, paused: bool) -> Result<()>;
    fn set_draining(&mut self);
    fn check_health(&self) -> Result<()>;
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

pub struct CpalAudioOutput;

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

struct QueueState {
    origin: Instant,
    consumed: AtomicU64,
    discard_until: AtomicU64,
    device_until_ns: AtomicU64,
    heartbeat_ns: AtomicU64,
    request_frames: AtomicUsize,
    starvations: AtomicUsize,
    monitoring: AtomicBool,
    failed: AtomicBool,
}

impl QueueState {
    fn new() -> Self {
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

struct CallbackQueue<T> {
    consumer: Consumer<T>,
    consumed: u64,
    state: Arc<QueueState>,
    channels: usize,
    rate: u32,
}

impl<T: AudioOutputSample> CallbackQueue<T> {
    fn render(&mut self, data: &mut [T], playback_delay: WallDuration) {
        let now = self.state.now_ns();
        self.state.heartbeat_ns.store(now, Ordering::Relaxed);
        self.state
            .request_frames
            .fetch_max(data.len().div_ceil(self.channels), Ordering::Relaxed);
        // Only the consumer advances the read index, including on seek while paused.
        let discard = self.state.discard_until.load(Ordering::Acquire);
        let skip = discard.saturating_sub(self.consumed) as usize;
        if skip > 0 {
            if let Ok(chunk) = self.consumer.read_chunk(skip) {
                chunk.commit_all();
                self.consumed += skip as u64;
            }
        }
        let requested = data.len() / self.channels * self.channels;
        let (written, _) = self.consumer.pop_partial_slice(&mut data[..requested]);
        let written = written.len();
        self.consumed += written as u64;
        if written > 0 {
            let duration = WallDuration::from_secs_f64(
                written as f64 / self.channels as f64 / self.rate as f64,
            );
            self.state.device_until_ns.store(
                now.saturating_add(playback_delay.as_nanos() as u64)
                    .saturating_add(duration.as_nanos() as u64),
                Ordering::Relaxed,
            );
        }
        // Publish the device deadline before removing these samples from pending_frames.
        self.state.consumed.store(self.consumed, Ordering::Release);
        if written < requested && self.state.monitoring.load(Ordering::Relaxed) {
            self.state.starvations.fetch_add(1, Ordering::Relaxed);
        }
        data[written..].fill(T::MID);
    }
}

trait AudioOutputSample: Sample + ConvertibleSample + Send + 'static {}

impl AudioOutputSample for f32 {}

impl AudioOutputSample for i32 {}

impl AudioOutputSample for i16 {}

impl AudioOutputSample for u16 {}

impl CpalAudioOutput {
    pub fn try_open(spec: AudioSpec, capacity: usize) -> Result<Box<dyn AudioOutput>> {
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

        // Select proper playback routine based on sample format.
        match config.sample_format() {
            cpal::SampleFormat::F32 => {
                CpalAudioOutputImpl::<f32>::try_open(spec, capacity, &device, default_device)
            }
            cpal::SampleFormat::I32 => {
                CpalAudioOutputImpl::<i32>::try_open(spec, capacity, &device, default_device)
            }
            cpal::SampleFormat::I16 => {
                CpalAudioOutputImpl::<i16>::try_open(spec, capacity, &device, default_device)
            }
            cpal::SampleFormat::U16 => {
                CpalAudioOutputImpl::<u16>::try_open(spec, capacity, &device, default_device)
            }
            format => Err(cpal::Error::with_message(
                cpal::ErrorKind::UnsupportedConfig,
                format!("Unsupported output sample format: {format}"),
            )
            .into()),
        }
    }
}

struct CpalAudioOutputImpl<T: AudioOutputSample>
where
    T: AudioOutputSample,
{
    channels: usize,
    sample_rate: u32,
    ring_buf_producer: Producer<T>,
    float_buf: Vec<f32>,
    sample_buf: Vec<T>,
    clip_mem: Vec<f32>,
    stream: cpal::Stream,
    state: Arc<QueueState>,
    produced: u64,
    policy: BufferPolicy,
    monitor_after: Option<Instant>,
    paused: bool,
    default_device: Option<DefaultDeviceMonitor>,
}

impl<T: AudioOutputSample + cpal::SizedSample> CpalAudioOutputImpl<T> {
    pub fn try_open(
        spec: AudioSpec,
        capacity: usize,
        device: &cpal::Device,
        default_device: Option<DefaultDeviceMonitor>,
    ) -> Result<Box<dyn AudioOutput>> {
        let num_channels = spec.channels().count();

        let requested_frames = (spec.rate() / 100).max(1);
        let buffer_size = device
            .supported_output_configs()
            .ok()
            .and_then(|configs| {
                configs
                    .filter(|config| {
                        config.channels() as usize == num_channels
                            && config.sample_format() == <T as cpal::SizedSample>::FORMAT
                            && config.min_sample_rate() <= spec.rate()
                            && config.max_sample_rate() >= spec.rate()
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
            sample_rate: SampleRate::from(spec.rate()),
            buffer_size,
        };
        let policy = BufferPolicy::new(spec.rate(), Instant::now());
        let (ring_buf_producer, consumer) = RingBuffer::new(policy.maximum * num_channels);
        let state = Arc::new(QueueState::new());
        let mut callback = CallbackQueue {
            consumer,
            consumed: 0,
            state: state.clone(),
            channels: num_channels,
            rate: spec.rate(),
        };
        let error_state = state.clone();
        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
                let timestamp = info.timestamp();
                callback.render(data, timestamp.playback.duration_since(timestamp.callback));
            },
            move |err| error_state.stream_error(err.kind()),
            Some(STALL_TIMEOUT),
        )?;

        // Start the output stream.
        stream.play()?;

        let float_buf = Vec::with_capacity(capacity * num_channels);
        let sample_buf = Vec::with_capacity(capacity * num_channels);

        Ok(Box::new(CpalAudioOutputImpl {
            channels: num_channels,
            sample_rate: spec.rate(),
            ring_buf_producer,
            float_buf,
            sample_buf,
            clip_mem: vec![0.0; num_channels],
            stream,
            state,
            produced: 0,
            policy,
            monitor_after: None,
            paused: false,
            default_device,
        }))
    }
}

impl<T: AudioOutputSample> AudioOutput for CpalAudioOutputImpl<T> {
    fn write(
        &mut self,
        decoded: GenericAudioBufferRef<'_>,
        volume: f32,
        skip_frames: usize,
    ) -> Result<()> {
        // Do nothing if there are no audio frames.
        if skip_frames >= decoded.frames() {
            return Ok(());
        }
        if self.paused {
            return Err(cpal::Error::new(cpal::ErrorKind::InvalidInput).into());
        }

        // Audio samples must be interleaved for cpal. Interleave the samples in the audio
        // buffer into the sample buffer.
        decoded.copy_to_vec_interleaved(&mut self.float_buf);

        let samples = &mut self.float_buf[skip_frames * self.channels..];
        for sample in samples.iter_mut() {
            *sample *= volume;
        }
        soft_clip(samples, self.channels, &mut self.clip_mem);

        self.sample_buf.clear();
        self.sample_buf.extend(
            samples
                .iter()
                .map(|&sample| <T as FromSample<f32>>::from_sample(sample)),
        );

        // Write all the interleaved samples to the ring buffer.
        let mut offset = 0;
        let monitor_after = *self
            .monitor_after
            .get_or_insert(Instant::now() + STARTUP_GRACE);
        let mut last_progress = Instant::now();
        while offset < self.sample_buf.len() {
            self.check_health()?;
            let now = Instant::now();
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
            let queued = self.policy.maximum * self.channels - self.ring_buf_producer.slots();
            // Backpressure only above the target: successive decoded packets fill toward it.
            // A lower target drains naturally; it never discards playable samples.
            let available = (self.policy.target * self.channels).saturating_sub(queued);
            let count = available.min(self.sample_buf.len() - offset);
            if count == 0 {
                if now.duration_since(last_progress) >= STALL_TIMEOUT {
                    return Err(AudioOutputError::StreamClosedError);
                }
                std::thread::sleep(WallDuration::from_millis(1));
                continue;
            }
            let (written, _) = self
                .ring_buf_producer
                .push_partial_slice(&self.sample_buf[offset..offset + count]);
            offset += written.len();
            self.produced += written.len() as u64;
            last_progress = now;
        }

        Ok(())
    }

    fn pending_frames(&self) -> usize {
        self.state
            .pending_frames(self.produced, self.channels, self.sample_rate)
    }

    fn discard_queued(&mut self) {
        self.state.monitoring.store(false, Ordering::Relaxed);
        self.state
            .discard_until
            .store(self.produced, Ordering::Release);
        self.state.starvations.store(0, Ordering::Relaxed);
        self.monitor_after = None;
        self.policy.reset(Instant::now());
        self.clip_mem.fill(0.0);
    }

    fn set_paused(&mut self, paused: bool) -> Result<()> {
        self.state.monitoring.store(false, Ordering::Relaxed);
        self.state.starvations.store(0, Ordering::Relaxed);
        self.monitor_after = None;
        self.policy.reset(Instant::now());
        self.state
            .heartbeat_ns
            .store(self.state.now_ns(), Ordering::Relaxed);
        if paused {
            self.stream.pause()
        } else {
            self.stream.play()
        }?;
        self.paused = paused;
        Ok(())
    }

    fn set_draining(&mut self) {
        self.state.monitoring.store(false, Ordering::Relaxed);
    }

    fn check_health(&self) -> Result<()> {
        if self.state.failed.load(Ordering::Relaxed)
            || self.ring_buf_producer.is_abandoned()
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

pub fn try_open(spec: AudioSpec, capacity: usize) -> Result<Box<dyn AudioOutput>> {
    CpalAudioOutput::try_open(spec, capacity)
}
