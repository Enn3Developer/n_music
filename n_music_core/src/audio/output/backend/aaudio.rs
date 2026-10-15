//! The output through AAudio, on Android. Streams ask for the power-saving path, the deep buffer
//! mixer, which wakes the processor less often than the default one. With offload allowed, from
//! Android 16 they first ask to play from a hardware buffer that holds seconds of audio, so the
//! processor sleeps in between.
//!
//! AAudio can't list devices. The app lists them with Android's AudioManager and hands them over
//! with [`set_devices`], and a stream opens on the chosen one's id while it is there.

use super::{Backend, BackendError, ErrorKind, Opened, Request, Shape, Stream};
use crate::audio::output::callback::{Renderer, SharedState};
use crate::audio::output::OutputFormat;
use crate::settings::OutputDevice;
use ndk::audio::{
    AudioCallbackResult, AudioContentType, AudioDirection, AudioError, AudioFormat,
    AudioPerformanceMode, AudioSharingMode, AudioStream, AudioStreamBuilder, AudioStreamState,
    AudioUsage, Clockid,
};
use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration as WallDuration, Instant};

/// `AAUDIO_PERFORMANCE_MODE_POWER_SAVING_OFFLOADED`, from API 36, which ndk 0.9 predates.
const POWER_SAVING_OFFLOADED: i32 = 13;
/// The API level offloaded streams came with.
const OFFLOAD_API: i32 = 36;
/// What offloaded streams are asked to play at: the rate the hardware itself runs at.
const OFFLOAD_RATE: i32 = 48_000;
const CHANNELS: i32 = 2;
/// The longest wait for a stream to settle in a state, like paused before a flush.
const STATE_TIMEOUT: WallDuration = WallDuration::from_millis(500);

extern "C" {
    /// In bionic from API 29.
    fn android_get_device_api_level() -> libc::c_int;
}

/// An output Android lists, as AudioManager tells it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidOutput {
    /// AudioDeviceInfo's id, which AAudio opens a device by. A device gets a new one each time
    /// it connects.
    pub id: i32,
    /// Tells the device apart across connections and launches: an [`OutputDevice::id`].
    pub key: String,
    pub name: String,
}

static DEVICES: RwLock<Vec<AndroidOutput>> = RwLock::new(Vec::new());

/// The outputs there are to play on, as AudioManager lists them now. Streams already open stay
/// where they are until they are opened again.
pub fn set_devices(devices: Vec<AndroidOutput>) {
    *DEVICES
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = devices;
}

/// The AAudio id of the output `key` names, while it is there.
fn device_id(key: &str) -> Option<i32> {
    DEVICES
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .find(|device| device.key == key)
        .map(|device| device.id)
}

impl From<AudioError> for BackendError {
    fn from(error: AudioError) -> Self {
        let kind = match error {
            AudioError::Disconnected => ErrorKind::Invalidated,
            AudioError::Unavailable | AudioError::NoService | AudioError::InvalidHandle => {
                ErrorKind::Unavailable
            }
            AudioError::NoFreeHandles | AudioError::NoMemory => ErrorKind::Busy,
            AudioError::IllegalArgument
            | AudioError::InvalidFormat
            | AudioError::InvalidRate
            | AudioError::OutOfRange
            | AudioError::Unimplemented => ErrorKind::Unsupported,
            _ => ErrorKind::Backend,
        };
        BackendError::new(kind, format!("AAudio: {error}"))
    }
}

fn offloaded() -> AudioPerformanceMode {
    AudioPerformanceMode::from(POWER_SAVING_OFFLOADED)
}

/// Closes `stream`. ndk panics when closing fails, but a stream that won't close is gone all the
/// same.
fn close(stream: AudioStream) {
    if std::panic::catch_unwind(AssertUnwindSafe(move || drop(stream))).is_err() {
        log::warn!("Could not close an AAudio stream");
    }
}

/// A way to open the stream, tried in order until one opens.
#[derive(Clone, Copy, Debug)]
struct Attempt {
    mode: AudioPerformanceMode,
    format: AudioFormat,
    /// `None` lets AAudio pick the device's own rate.
    rate: Option<i32>,
}

fn attempts(offload: bool) -> Vec<Attempt> {
    let formats = [AudioFormat::PCM_Float, AudioFormat::PCM_I16];
    let mut attempts = vec![];
    // SAFETY: a plain query, in libc on every API level the app runs on.
    if offload && unsafe { android_get_device_api_level() } >= OFFLOAD_API {
        attempts.extend(formats.map(|format| Attempt {
            mode: offloaded(),
            format,
            rate: Some(OFFLOAD_RATE),
        }));
    }
    for mode in [
        AudioPerformanceMode::PowerSaving,
        AudioPerformanceMode::None,
    ] {
        attempts.extend(formats.map(|format| Attempt {
            mode,
            format,
            rate: None,
        }));
    }
    attempts
}

/// What the stream callback renders with, once the stream is open.
type Slot = Arc<Mutex<Option<Renderer>>>;

pub(in crate::audio::output) struct AAudio;

impl Backend for AAudio {
    type Stream = AAudioStream;

    fn devices() -> Vec<OutputDevice> {
        DEVICES
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .map(|device| OutputDevice {
                id: device.key.clone(),
                name: device.name.clone(),
            })
            .collect()
    }

    fn open(
        request: &Request,
        renderer: impl FnOnce(&Shape) -> Renderer,
    ) -> Result<Opened<AAudioStream>, BackendError> {
        let chosen = request.device.and_then(|key| {
            let id = device_id(key);
            if id.is_none() {
                log::info!("Output device {key} is not there, playing on the default");
            }
            id
        });
        let mut last = None;
        // The chosen device first; the default if it can't play.
        for device in chosen.map(Some).into_iter().chain([None]) {
            for attempt in attempts(request.offload) {
                let slot = Slot::default();
                match open_stream(attempt, device, slot.clone(), request.state.clone()) {
                    // A device without offload may open the stream on another path: the
                    // power-saving one is the better of those.
                    Ok(stream)
                        if attempt.mode == offloaded()
                            && stream.performance_mode() != offloaded() =>
                    {
                        log::debug!("AAudio opened {attempt:?} on another path; trying the next");
                        close(stream);
                    }
                    Ok(stream) => return Ok(opened(stream, slot, renderer)),
                    Err(error) => {
                        log::debug!("AAudio refused {attempt:?} on {device:?}: {error}");
                        last = Some(error);
                    }
                }
            }
        }
        Err(last.map_or_else(
            || BackendError::new(ErrorKind::Other, "AAudio: nothing to try"),
            BackendError::from,
        ))
    }
}

fn open_stream(
    attempt: Attempt,
    device: Option<i32>,
    slot: Slot,
    state: Arc<SharedState>,
) -> Result<AudioStream, AudioError> {
    let mut builder = AudioStreamBuilder::new()?
        .direction(AudioDirection::Output)
        .sharing_mode(AudioSharingMode::Shared)
        .performance_mode(attempt.mode)
        .format(attempt.format)
        .channel_count(CHANNELS)
        .usage(AudioUsage::Media)
        .content_type(AudioContentType::Music);
    if let Some(rate) = attempt.rate {
        builder = builder.sample_rate(rate);
    }
    if let Some(device) = device {
        builder = builder.device_id(device);
    }
    let mut xruns = 0;
    let glitches = state.clone();
    builder
        .data_callback(Box::new(move |stream, data, frames| {
            render(stream, &slot, data, frames);
            let count = stream.x_run_count();
            if count > xruns {
                xruns = count;
                glitches.glitch();
            }
            AudioCallbackResult::Continue
        }))
        // Disconnected streams don't come back: the writer opens another. It must not happen
        // here, on AAudio's thread.
        .error_callback(Box::new(move |_, _| state.fail()))
        .open_stream()
}

/// Fills the stream's buffer, silence until the stream has its renderer.
fn render(stream: &AudioStream, slot: &Slot, data: *mut c_void, frames: i32) {
    let samples = frames.max(0) as usize * stream.channel_count().max(0) as usize;
    let format = stream.format();
    let renderer = slot.try_lock();
    let mut renderer = match renderer {
        Ok(renderer) => renderer,
        Err(_) => return silence(data, samples, format),
    };
    let Some(renderer) = renderer.as_mut() else {
        return silence(data, samples, format);
    };
    let delay = playback_delay(stream);
    match format {
        AudioFormat::PCM_Float => {
            // SAFETY: AAudio hands over room for `frames` frames of the stream's format.
            let data = unsafe { std::slice::from_raw_parts_mut(data.cast::<f32>(), samples) };
            renderer.render(data, delay);
        }
        AudioFormat::PCM_I16 => {
            // SAFETY: as above.
            let data = unsafe { std::slice::from_raw_parts_mut(data.cast::<i16>(), samples) };
            renderer.render(data, delay);
        }
        _ => silence(data, samples, format),
    }
}

fn silence(data: *mut c_void, samples: usize, format: AudioFormat) {
    let size = match format {
        AudioFormat::PCM_I16 => 2,
        _ => 4,
    };
    // SAFETY: AAudio hands over room for this many samples; zero bytes are silence in both
    // formats the stream is opened with.
    unsafe { std::ptr::write_bytes(data.cast::<u8>(), 0, samples * size) };
}

/// How long until the device plays the first frame the callback writes now: the last frame it
/// reported playing, moved on to the frames written since.
fn playback_delay(stream: &AudioStream) -> WallDuration {
    let rate = i128::from(stream.sample_rate());
    let Ok(timestamp) = stream.timestamp(Clockid::Monotonic) else {
        return WallDuration::ZERO;
    };
    if rate <= 0 {
        return WallDuration::ZERO;
    }
    let ahead = i128::from(stream.frames_written()) - i128::from(timestamp.frame_position);
    let plays_at = i128::from(timestamp.time_nanoseconds) + ahead * 1_000_000_000 / rate;
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: writes the time into `now`.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut now) } != 0 {
        return WallDuration::ZERO;
    }
    let now = i128::from(now.tv_sec) * 1_000_000_000 + i128::from(now.tv_nsec);
    WallDuration::from_nanos((plays_at - now).clamp(0, i128::from(u64::MAX)) as u64)
}

/// The stream `stream` with its renderer in place, and its shape.
fn opened(
    stream: AudioStream,
    slot: Slot,
    renderer: impl FnOnce(&Shape) -> Renderer,
) -> Opened<AAudioStream> {
    let deep = stream.performance_mode() == offloaded();
    let format = OutputFormat {
        rate: stream.sample_rate().max(1) as u32,
        channels: stream.channel_count().max(1) as usize,
    };
    let device_frames = stream
        .buffer_capacity_in_frames()
        .max(stream.frames_per_burst())
        .max(0) as usize;
    let shape = Shape {
        format,
        deep,
        device_frames,
    };
    log::info!(
        "AAudio stream on device {}: {:?}, {:?}, burst {}, capacity {}",
        stream.device_id(),
        stream.performance_mode(),
        stream.format(),
        stream.frames_per_burst(),
        stream.buffer_capacity_in_frames(),
    );
    *slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(renderer(&shape));
    Opened {
        stream: AAudioStream {
            stream: Some(stream),
        },
        shape,
    }
}

pub(in crate::audio::output) struct AAudioStream {
    /// `None` only while dropping.
    stream: Option<AudioStream>,
}

// SAFETY: AAudio takes calls on a stream from any thread, one at a time. The output makes them
// from the thread that holds it, and the callbacks only reach the renderer's slot and the shared
// state, which are thread safe.
unsafe impl Send for AAudioStream {}

impl AAudioStream {
    fn stream(&self) -> &AudioStream {
        self.stream
            .as_ref()
            .expect("the stream is only taken while dropping")
    }

    /// Waits while the stream is in `passing`, a state on the way to another.
    fn settle(&self, passing: AudioStreamState) -> Result<AudioStreamState, BackendError> {
        let stream = self.stream();
        let deadline = Instant::now() + STATE_TIMEOUT;
        let mut state = stream.state();
        while state == passing {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            state = stream.wait_for_state_change(passing, left.as_nanos() as i64)?;
        }
        Ok(state)
    }
}

impl Stream for AAudioStream {
    fn play(&mut self) -> Result<(), BackendError> {
        Ok(self.stream().request_start()?)
    }

    fn pause(&mut self) -> Result<(), BackendError> {
        Ok(self.stream().request_pause()?)
    }

    fn flush(&mut self) -> Result<(), BackendError> {
        match self.stream().state() {
            // Never started, or already empty: nothing to drop.
            AudioStreamState::Open | AudioStreamState::Flushed => return Ok(()),
            AudioStreamState::Paused => {}
            AudioStreamState::Pausing => {
                self.settle(AudioStreamState::Pausing)?;
            }
            _ => {
                self.stream().request_pause()?;
                self.settle(AudioStreamState::Pausing)?;
            }
        }
        self.stream().request_flush()?;
        self.settle(AudioStreamState::Flushing)?;
        Ok(())
    }
}

impl Drop for AAudioStream {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.take() {
            close(stream);
        }
    }
}
