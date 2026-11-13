//! The output through Core Audio, on macOS: an AUHAL unit (`kAudioUnitSubType_HALOutput`) on the
//! chosen or the default device, fed interleaved `f32` at the device's own rate.
//!
//! Before opening, the device is switched to the rate of what plays where it has it (or a whole
//! multiple of it), see [`rate`]. The I/O buffer is 10 ms, or 40 ms while power saving is asked
//! for, so the processor wakes less often. The unit stops while paused. Where the stream should
//! play is followed through the HAL's property listeners, see [`watch`].

mod hal;
mod rate;
mod watch;

use super::{Backend, BackendError, ErrorKind, Opened, Request, Shape, Stream};
use crate::audio::output::callback::Renderer;
use crate::audio::output::OutputFormat;
use crate::settings::OutputDevice;
use hal::DeviceId;
use objc2_audio_toolbox::{
    kAudioOutputUnitProperty_CurrentDevice, kAudioUnitManufacturer_Apple,
    kAudioUnitProperty_MaximumFramesPerSlice, kAudioUnitProperty_SetRenderCallback,
    kAudioUnitProperty_StreamFormat, kAudioUnitScope_Global, kAudioUnitScope_Input,
    kAudioUnitScope_Output, kAudioUnitSubType_HALOutput, kAudioUnitType_Output,
    AURenderCallbackStruct, AudioComponentDescription, AudioComponentFindNext,
    AudioComponentInstanceDispose, AudioComponentInstanceNew, AudioOutputUnitStart,
    AudioOutputUnitStop, AudioUnit, AudioUnitGetProperty, AudioUnitInitialize, AudioUnitPropertyID,
    AudioUnitRenderActionFlags, AudioUnitScope, AudioUnitSetProperty, AudioUnitUninitialize,
};
use objc2_core_audio::{AudioConvertHostTimeToNanos, AudioGetCurrentHostTime};
use objc2_core_audio_types::{
    kAudioFormatFlagsNativeFloatPacked, kAudioFormatLinearPCM, AudioBufferList,
    AudioStreamBasicDescription, AudioTimeStamp, AudioTimeStampFlags,
};
use std::ffi::c_void;
use std::mem::{size_of, MaybeUninit};
use std::panic::AssertUnwindSafe;
use std::ptr::{null_mut, NonNull};
use std::time::Duration;
use watch::Watch;

/// The I/O buffer: how much the device asks for at once.
const BUFFER: Duration = Duration::from_millis(10);
/// The I/O buffer while power saving is asked for.
const POWER_SAVING_BUFFER: Duration = Duration::from_millis(40);
/// The most frames a render call may ask for, as a share of a second: never more than the
/// renderer mixes without allocating (the output queues half a second).
const MAX_SLICE_SECONDS: f64 = 0.25;

pub(in crate::audio::output) struct CoreAudio;

/// The UID of the device `key` names, an [`OutputDevice::id`].
fn chosen_uid(key: &str) -> Option<&str> {
    let uid = hal::uid_of(key);
    if uid.is_none() {
        log::warn!("Ignoring the output device {key:?}: not a Core Audio device");
    }
    uid
}

impl Backend for CoreAudio {
    type Stream = CoreAudioStream;

    const SWITCHES_RATE: bool = true;

    fn devices() -> Vec<OutputDevice> {
        hal::devices()
            .into_iter()
            .filter(|&device| hal::has_output(device))
            .filter_map(|device| {
                let uid = hal::uid(device)?;
                let name = hal::name(device).unwrap_or_else(|| uid.clone());
                Some(OutputDevice {
                    id: hal::device_key(&uid),
                    name,
                })
            })
            .collect()
    }

    fn rate_for(device: Option<&str>, wanted: u32) -> Option<u32> {
        let device = hal::resolve(device.and_then(hal::uid_of))?;
        rate::target(device, wanted).ok().map(|(_, rate)| rate)
    }

    fn open(
        request: &Request,
        renderer: impl FnOnce(&Shape) -> Renderer,
    ) -> Result<Opened<CoreAudioStream>, BackendError> {
        let chosen = request.device.and_then(chosen_uid);
        let found = chosen.and_then(hal::by_uid);
        if let (Some(chosen), None) = (chosen, found) {
            log::info!("Output device {chosen} is not there, playing on the default");
        }
        let device = found
            .or_else(hal::default_output)
            .ok_or_else(|| BackendError::new(ErrorKind::Unavailable, "No output device"))?;
        let rate = match request.rate {
            Some(wanted) => rate::target(device, wanted)
                .and_then(|(current, target)| rate::switch(device, current, target))
                .map_err(|status| hal::error("switching the device's rate", status)),
            // Without a rate to match, as once matching is turned off, the device goes back to
            // the rate it ran at before matching switched it.
            None => rate::restore(device)
                .and_then(|()| hal::nominal_rate(device))
                .map_err(|status| hal::error("reading the device's rate", status)),
        }
        .map_err(|error| gone(error, device, chosen))?;
        let mut stream = CoreAudioStream::new()?;
        stream
            .configure(device, rate, request, renderer)
            .map_err(|error| gone(error, device, chosen))?;
        stream.watch = Some(Watch::new(
            device,
            chosen.map(str::to_owned),
            request.state.clone(),
        ));
        let shape = stream.shape.ok_or_else(|| {
            BackendError::new(ErrorKind::Other, "Core Audio: the output has no shape")
        })?;
        Ok(Opened { stream, shape })
    }
}

/// `error` from opening on `device` as the device going away, when it did or the output should
/// be on another one now: the AudioUnit errors a missing device gives read as unsupported
/// formats, which opening again would not fix. Opening again finds the device there is now.
fn gone(mut error: BackendError, device: DeviceId, chosen: Option<&str>) -> BackendError {
    if error.kind != ErrorKind::Invalidated
        && (!hal::is_alive(device) || hal::resolve(chosen) != Some(device))
    {
        log::info!("The output device went away while opening: {error}");
        error.kind = ErrorKind::Invalidated;
    }
    error
}

/// What the render callback works with; only its thread touches it once the unit is set up.
struct Callback {
    renderer: Renderer,
    channels: usize,
    /// How long after a buffer's output time the device plays it: its latency and its stream's.
    latency: Duration,
}

/// AUHAL's render callback, on the device's realtime thread: fills `data` from the renderer,
/// silence if it panics. Nothing here blocks or allocates.
unsafe extern "C-unwind" fn render(
    refcon: NonNull<c_void>,
    _flags: NonNull<AudioUnitRenderActionFlags>,
    timestamp: NonNull<AudioTimeStamp>,
    _bus: u32,
    frames: u32,
    data: *mut AudioBufferList,
) -> i32 {
    let rendered = std::panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `refcon` is the stream's callback, which outlives the unit; the HAL hands over
        // a valid timestamp and buffer list.
        unsafe {
            fill(
                refcon.cast::<Callback>().as_mut(),
                timestamp.as_ref(),
                frames,
                data,
            )
        }
    }));
    if rendered.is_err() {
        // SAFETY: as above.
        unsafe { silence(data) };
    }
    0
}

/// # Safety
///
/// `data` must be null or a buffer list whose buffers hold `mDataByteSize` bytes each.
unsafe fn fill(
    callback: &mut Callback,
    timestamp: &AudioTimeStamp,
    frames: u32,
    data: *mut AudioBufferList,
) {
    // SAFETY: as the caller says.
    let Some(list) = (unsafe { data.as_mut() }) else {
        return;
    };
    // SAFETY: the list holds `mNumberBuffers` buffers, past the one declared inline.
    let buffers = unsafe {
        std::slice::from_raw_parts_mut(list.mBuffers.as_mut_ptr(), list.mNumberBuffers as usize)
    };
    let Some((first, others)) = buffers.split_first_mut() else {
        return;
    };
    for buffer in others {
        // SAFETY: as the caller says.
        unsafe { zero(buffer.mData, buffer.mDataByteSize) };
    }
    if first.mData.is_null() {
        return;
    }
    let samples =
        (first.mDataByteSize as usize / size_of::<f32>()).min(frames as usize * callback.channels);
    // SAFETY: the buffer holds `mDataByteSize` bytes of interleaved f32, as the stream format
    // asks for.
    let out = unsafe { std::slice::from_raw_parts_mut(first.mData.cast::<f32>(), samples) };
    callback
        .renderer
        .render(out, delay(timestamp, callback.latency));
}

/// How long until the device plays the first frame of the buffer with `timestamp`.
fn delay(timestamp: &AudioTimeStamp, latency: Duration) -> Duration {
    if !timestamp
        .mFlags
        .contains(AudioTimeStampFlags::HostTimeValid)
    {
        return latency;
    }
    // SAFETY: plain clock reads.
    let now = unsafe { AudioGetCurrentHostTime() };
    let ahead = timestamp.mHostTime.saturating_sub(now);
    if ahead == 0 {
        return latency;
    }
    // SAFETY: a plain conversion.
    latency + Duration::from_nanos(unsafe { AudioConvertHostTimeToNanos(ahead) })
}

/// # Safety
///
/// `data` must be null or a valid buffer list.
unsafe fn silence(data: *mut AudioBufferList) {
    // SAFETY: as the caller says.
    let Some(list) = (unsafe { data.as_mut() }) else {
        return;
    };
    // SAFETY: as in `fill`.
    let buffers = unsafe {
        std::slice::from_raw_parts_mut(list.mBuffers.as_mut_ptr(), list.mNumberBuffers as usize)
    };
    for buffer in buffers {
        // SAFETY: as the caller says.
        unsafe { zero(buffer.mData, buffer.mDataByteSize) };
    }
}

/// # Safety
///
/// `data` must be null or hold `size` writable bytes.
unsafe fn zero(data: *mut c_void, size: u32) {
    if !data.is_null() {
        // SAFETY: as the caller says; zero bytes are silence in f32.
        unsafe { std::ptr::write_bytes(data.cast::<u8>(), 0, size as usize) };
    }
}

fn set_property<T>(
    unit: AudioUnit,
    property: AudioUnitPropertyID,
    scope: AudioUnitScope,
    value: &T,
    what: &str,
) -> Result<(), BackendError> {
    // SAFETY: the unit reads `size_of::<T>()` bytes of `value`, of the type the property takes.
    let status = unsafe {
        AudioUnitSetProperty(
            unit,
            property,
            scope,
            0,
            (value as *const T).cast(),
            size_of::<T>() as u32,
        )
    };
    hal::check(status, what)
}

fn get_property<T: hal::Plain>(
    unit: AudioUnit,
    property: AudioUnitPropertyID,
    scope: AudioUnitScope,
    what: &str,
) -> Result<T, BackendError> {
    let mut value = MaybeUninit::<T>::zeroed();
    let mut size = size_of::<T>() as u32;
    // SAFETY: `value` has room for `size` bytes, which is all the unit writes.
    let status = unsafe {
        AudioUnitGetProperty(
            unit,
            property,
            scope,
            0,
            NonNull::from(&mut value).cast(),
            NonNull::from(&mut size),
        )
    };
    hal::check(status, what)?;
    // SAFETY: zeroed, then written by the unit: valid either way for a `Plain` type.
    Ok(unsafe { value.assume_init() })
}

/// An AUHAL unit and what its callback renders with. Dropping it stops and disposes of the unit.
pub(in crate::audio::output) struct CoreAudioStream {
    unit: AudioUnit,
    /// Made once the unit is set up; freed only after the unit is disposed of.
    callback: Option<NonNull<Callback>>,
    watch: Option<Watch>,
    shape: Option<Shape>,
    initialized: bool,
    running: bool,
}

// SAFETY: a unit takes calls from any thread, one at a time: the output makes them from the
// thread that holds it. The callback is only touched by the unit's render thread.
unsafe impl Send for CoreAudioStream {}

impl CoreAudioStream {
    fn new() -> Result<Self, BackendError> {
        let description = AudioComponentDescription {
            componentType: kAudioUnitType_Output,
            componentSubType: kAudioUnitSubType_HALOutput,
            componentManufacturer: kAudioUnitManufacturer_Apple,
            componentFlags: 0,
            componentFlagsMask: 0,
        };
        // SAFETY: looks the component up from the start.
        let component = unsafe { AudioComponentFindNext(null_mut(), NonNull::from(&description)) };
        if component.is_null() {
            return Err(BackendError::new(
                ErrorKind::Unavailable,
                "Core Audio: no HAL output unit",
            ));
        }
        let mut unit: AudioUnit = null_mut();
        // SAFETY: writes the new instance into `unit`.
        let status = unsafe { AudioComponentInstanceNew(component, NonNull::from(&mut unit)) };
        hal::check(status, "creating the output unit")?;
        if unit.is_null() {
            return Err(BackendError::new(
                ErrorKind::Backend,
                "Core Audio: no output unit was made",
            ));
        }
        Ok(Self {
            unit,
            callback: None,
            watch: None,
            shape: None,
            initialized: false,
            running: false,
        })
    }

    /// Sets the unit up on `device`, running at `rate` Hz, rendering with what `renderer` makes.
    fn configure(
        &mut self,
        device: DeviceId,
        rate: u32,
        request: &Request,
        renderer: impl FnOnce(&Shape) -> Renderer,
    ) -> Result<(), BackendError> {
        let unit = self.unit;
        set_property(
            unit,
            kAudioOutputUnitProperty_CurrentDevice,
            kAudioUnitScope_Global,
            &device,
            "choosing the output device",
        )?;
        // What the device side of the unit plays: its channels.
        let device_format: AudioStreamBasicDescription = get_property(
            unit,
            kAudioUnitProperty_StreamFormat,
            kAudioUnitScope_Output,
            "reading the device's format",
        )?;
        let channels = device_format.mChannelsPerFrame;
        if channels == 0 || rate == 0 {
            return Err(BackendError::new(
                ErrorKind::Unsupported,
                format!("Core Audio: the device plays {channels} channels at {rate} Hz"),
            ));
        }

        let buffer = if request.offload {
            POWER_SAVING_BUFFER
        } else {
            BUFFER
        };
        let wanted = ((f64::from(rate) * buffer.as_secs_f64()).round() as u32).max(1);
        let range = hal::buffer_frame_range(device);
        let frames = range.map_or(wanted, |(min, max)| wanted.clamp(min, max.max(min)));
        if let Err(status) = hal::set_buffer_frame_size(device, frames) {
            log::warn!(
                "Could not set the I/O buffer to {frames} frames: {}",
                hal::describe(status)
            );
        }
        let frames = hal::buffer_frame_size(device).unwrap_or(frames);
        let max_slice = range
            .map_or(frames, |(_, max)| max)
            .min((f64::from(rate) * MAX_SLICE_SECONDS) as u32)
            .max(frames);
        set_property(
            unit,
            kAudioUnitProperty_MaximumFramesPerSlice,
            kAudioUnitScope_Global,
            &max_slice,
            "setting the most frames per render",
        )?;

        let bytes_per_frame = channels * size_of::<f32>() as u32;
        let format = AudioStreamBasicDescription {
            mSampleRate: f64::from(rate),
            mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kAudioFormatFlagsNativeFloatPacked,
            mBytesPerPacket: bytes_per_frame,
            mFramesPerPacket: 1,
            mBytesPerFrame: bytes_per_frame,
            mChannelsPerFrame: channels,
            mBitsPerChannel: 32,
            mReserved: 0,
        };
        set_property(
            unit,
            kAudioUnitProperty_StreamFormat,
            kAudioUnitScope_Input,
            &format,
            "setting the stream format",
        )?;

        let (latency, safety, stream_latency) = hal::latencies(device);
        let shape = Shape {
            format: OutputFormat {
                rate,
                channels: channels as usize,
            },
            deep: false,
            device_frames: frames as usize,
        };
        let callback = Box::new(Callback {
            renderer: renderer(&shape),
            channels: channels as usize,
            latency: Duration::from_secs_f64(f64::from(latency + stream_latency) / f64::from(rate)),
        });
        let callback = NonNull::from(Box::leak(callback));
        self.callback = Some(callback);
        let render_callback = AURenderCallbackStruct {
            inputProc: Some(render),
            inputProcRefCon: callback.as_ptr().cast(),
        };
        set_property(
            unit,
            kAudioUnitProperty_SetRenderCallback,
            kAudioUnitScope_Input,
            &render_callback,
            "setting the render callback",
        )?;
        // SAFETY: a unit made by `new`, set up above.
        hal::check(
            unsafe { AudioUnitInitialize(unit) },
            "starting up the output unit",
        )?;
        self.initialized = true;
        self.shape = Some(shape);

        let total = latency + safety + stream_latency + frames;
        let ms = |frames: u32| f64::from(frames) * 1000.0 / f64::from(rate);
        log::info!(
            "Core Audio output on {}: {rate} Hz, {channels} channels, I/O buffer {frames} frames \
             ({:.1} ms), latency {:.1} ms (device {latency} + safety offset {safety} + stream \
             {stream_latency} + buffer {frames} frames)",
            hal::name(device).unwrap_or_default(),
            ms(frames),
            ms(total),
        );
        Ok(())
    }
}

impl Stream for CoreAudioStream {
    fn play(&mut self) -> Result<(), BackendError> {
        if !self.running {
            // SAFETY: an initialized unit.
            hal::check(
                unsafe { AudioOutputUnitStart(self.unit) },
                "starting the output",
            )?;
            self.running = true;
        }
        Ok(())
    }

    fn pause(&mut self) -> Result<(), BackendError> {
        if self.running {
            // SAFETY: as above. It returns once the render callback is done.
            hal::check(
                unsafe { AudioOutputUnitStop(self.unit) },
                "stopping the output",
            )?;
            self.running = false;
        }
        Ok(())
    }

    fn moved(&self) -> bool {
        self.watch.as_ref().is_some_and(Watch::moved)
    }
}

impl Drop for CoreAudioStream {
    fn drop(&mut self) {
        self.watch.take();
        // SAFETY: the unit made by `new`, stopped before it is disposed of; nothing calls the
        // render callback once it is.
        unsafe {
            if self.running {
                AudioOutputUnitStop(self.unit);
            }
            if self.initialized {
                AudioUnitUninitialize(self.unit);
            }
            AudioComponentInstanceDispose(self.unit);
        }
        if let Some(callback) = self.callback.take() {
            // SAFETY: leaked from a box in `configure`, and the unit that used it is gone.
            drop(unsafe { Box::from_raw(callback.as_ptr()) });
        }
    }
}

#[cfg(test)]
mod tests;
