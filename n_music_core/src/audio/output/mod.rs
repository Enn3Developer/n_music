//! The audio output: one long-lived stream to the chosen or the default device, fed with f32
//! frames in the device's own rate and channel layout. The stream callback applies the volume
//! (ramped, so changes, pauses and seeks do not click), soft-clips and converts to the device's
//! sample type. [`backend`] opens the stream on what the platform has.
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
//! A deep stream, like offloaded playback on Android, holds seconds of audio, too much to wait
//! for. Pausing it stops the device where it is, and seeking flushes what it holds. It starts
//! only once the queue holds what the device asks for at once, so its first request isn't
//! filled with silence.
//!
//! Started from [symphonia-play's `output.rs`](https://github.com/pdeljanov/Symphonia/blob/master/symphonia-play/src/output.rs)
//! by [Philip Deljanov](https://github.com/pdeljanov).

mod backend;
mod callback;
mod dsp;

#[cfg(target_os = "android")]
pub use backend::{set_devices, AndroidOutput};

use crate::settings::OutputDevice;
use backend::{Backend, BackendError, ErrorKind, Platform, Request, Shape, Stream};
use callback::{Renderer, SharedState};
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
    Backend(BackendError),
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

impl From<BackendError> for AudioOutputError {
    fn from(error: BackendError) -> Self {
        Self::Backend(error)
    }
}

impl AudioOutputError {
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::StreamClosedError => true,
            Self::Backend(error) => error.is_recoverable(),
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
/// A stream whose callback stopped coming for this long is taken for dead. A deep stream gets
/// twice what its device holds on top, as it asks for more only that often.
const STALL_TIMEOUT: WallDuration = WallDuration::from_secs(2);
pub const DEVICE_CHECK_INTERVAL: WallDuration = WallDuration::from_millis(500);
/// Audio queued ahead of the device. The writer refills it once half is played.
const QUEUE: WallDuration = WallDuration::from_millis(500);
/// The most a deep stream queues: what its device asks for at once, plus [`QUEUE`].
const DEEP_QUEUE_LIMIT: WallDuration = WallDuration::from_secs(30);

/// The devices the output can play on, as the system lists them.
pub(crate) fn output_devices() -> Vec<OutputDevice> {
    Platform::devices()
}

/// The stream to the output device. Writing never blocks: callers wait with
/// [`Output::wait_for_room`], which the stream callback wakes up.
pub struct Output {
    format: OutputFormat,
    producer: Producer<f32>,
    stream: <Platform as Backend>::Stream,
    state: Arc<SharedState>,
    /// Samples ever written.
    produced: u64,
    /// The queue's size in samples.
    capacity: usize,
    paused: bool,
    /// See [`Shape::deep`].
    deep: bool,
    /// A deep stream waits for the queue to hold this many samples before it starts.
    prime: usize,
    /// A deep stream that was opened or flushed and has not started yet.
    priming: bool,
    stall_timeout: WallDuration,
}

impl Output {
    /// Opens `device`, an [`OutputDevice::id`], while it is there, else the default device, in
    /// its default configuration, playing at `volume`. `offload` lets the platform play from a
    /// deep hardware buffer where it can.
    pub fn open(volume: f32, device: Option<&str>, offload: bool) -> Result<Self> {
        let state = Arc::new(SharedState::new(volume));
        let request = Request {
            device,
            offload,
            state: state.clone(),
        };
        let mut queue = None;
        let opened = Platform::open(&request, |shape| {
            let frames = queue_frames(shape);
            let (producer, consumer) = RingBuffer::new(frames * shape.format.channels);
            queue = Some(producer);
            // A callback never gets more than the queue, so mixing never allocates.
            Renderer::new(consumer, state.clone(), shape.format, frames)
        })?;
        let producer = queue.ok_or_else(|| {
            BackendError::new(ErrorKind::Other, "The audio backend opened no queue")
        })?;
        let Shape {
            format,
            deep,
            device_frames,
        } = opened.shape;
        let capacity = producer.slots();
        // What the device asks for at once, as far as the queue can hold it.
        let device_frames = device_frames.min(capacity / format.channels);
        let device = WallDuration::from_secs_f64(device_frames as f64 / f64::from(format.rate));
        let mut output = Self {
            format,
            producer,
            stream: opened.stream,
            state,
            produced: 0,
            capacity,
            paused: false,
            deep,
            prime: device_frames * format.channels,
            priming: deep,
            stall_timeout: if deep {
                STALL_TIMEOUT + device * 2
            } else {
                STALL_TIMEOUT
            },
        };
        if !deep {
            output.stream.play()?;
        }
        log::info!(
            "Opened audio output: {} Hz, {} channels{}",
            format.rate,
            format.channels,
            if deep {
                format!(", deep, {device_frames} frames at once")
            } else {
                String::new()
            }
        );
        Ok(output)
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
        if self.priming && self.queued() >= self.prime {
            self.start()?;
        }
        Ok(written.len())
    }

    /// Starts a deep stream that was waiting for its queue to fill.
    fn start(&mut self) -> Result<()> {
        self.priming = false;
        self.state
            .heartbeat_ns
            .store(self.state.now_ns(), Ordering::Relaxed);
        self.state.release_device();
        self.stream.play()?;
        Ok(())
    }

    /// Parks the attached thread until half the queue is free, or `timeout`. Anyone holding
    /// the thread handle may also unpark it earlier.
    pub fn wait_for_room(&self, timeout: WallDuration) {
        let low = self.capacity / 2;
        self.park(low + 1, || self.queued() <= low, timeout);
    }

    /// Parks the attached thread until everything queued was played, or `timeout`. A deep
    /// stream still filling its queue starts: nothing more comes.
    pub fn wait_drained(&mut self, timeout: WallDuration) {
        if self.priming && !self.paused {
            if let Err(error) = self.start() {
                log::warn!("Could not start the audio output: {error}");
                self.state.fail();
                return;
            }
        }
        if self.queued() > 0 {
            self.park(1, || self.queued() == 0, timeout);
            return;
        }
        // Handed over: only the device is left, and no callback says when it played that.
        let left = self.state.device_left().min(timeout);
        if !left.is_zero() {
            std::thread::park_timeout(left);
        }
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

    /// Fades out, then drops everything queued. A deep stream drops what its device holds too,
    /// and starts again once the queue is refilled.
    pub fn discard_queued(&mut self) {
        if self.deep {
            let flushed = self.stream.flush();
            self.state
                .discard_until
                .store(self.produced, Ordering::Release);
            self.state.clear_device();
            self.priming = true;
            if let Err(error) = flushed {
                log::warn!("Could not flush the audio output: {error}");
                self.state.fail();
            }
            return;
        }
        self.fade_out();
        self.state
            .discard_until
            .store(self.produced, Ordering::Release);
        if !self.paused {
            self.state.muted.store(false, Ordering::Relaxed);
        }
    }

    /// Pausing fades out first and pauses the stream once the fade-out was heard; resuming
    /// fades back in. A deep stream pauses where its device is.
    pub fn set_paused(&mut self, paused: bool) -> Result<()> {
        if paused == self.paused {
            return Ok(());
        }
        self.state
            .heartbeat_ns
            .store(self.state.now_ns(), Ordering::Relaxed);
        if paused {
            if !self.deep {
                self.fade_out();
                self.play_out();
            }
            if !self.priming {
                self.stream.pause()?;
            }
            self.state.hold_device();
        } else {
            if !self.deep {
                self.state.muted.store(false, Ordering::Relaxed);
            }
            if !self.priming {
                self.state.release_device();
                self.stream.play()?;
            } else if self.queued() >= self.prime {
                self.paused = false;
                return self.start();
            }
        }
        self.paused = paused;
        Ok(())
    }

    pub fn check_health(&self) -> Result<()> {
        if self.state.failed.load(Ordering::Relaxed)
            || self.producer.is_abandoned()
            || self.stream.moved()
            || (!self.paused
                && !self.priming
                && self
                    .state
                    .now_ns()
                    .saturating_sub(self.state.heartbeat_ns.load(Ordering::Relaxed))
                    > self.stall_timeout.as_nanos() as u64)
        {
            Err(AudioOutputError::StreamClosedError)
        } else {
            Ok(())
        }
    }
}

/// How many frames to queue for a stream of `shape`.
fn queue_frames(shape: &Shape) -> usize {
    let rate = f64::from(shape.format.rate);
    let frames = (rate * QUEUE.as_secs_f64()) as usize;
    let frames = if shape.deep {
        frames
            .saturating_add(shape.device_frames)
            .min((rate * DEEP_QUEUE_LIMIT.as_secs_f64()) as usize)
    } else {
        frames
    };
    frames.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(deep: bool, device_frames: usize) -> Shape {
        Shape {
            format: OutputFormat {
                rate: 48_000,
                channels: 2,
            },
            deep,
            device_frames,
        }
    }

    #[test]
    fn a_shallow_stream_queues_half_a_second() {
        assert_eq!(queue_frames(&shape(false, 480)), 24_000);
    }

    #[test]
    fn a_deep_stream_queues_what_its_device_asks_for_on_top() {
        assert_eq!(queue_frames(&shape(true, 4 * 48_000)), 24_000 + 4 * 48_000);
        assert_eq!(queue_frames(&shape(true, 600 * 48_000)), 30 * 48_000);
    }
}
