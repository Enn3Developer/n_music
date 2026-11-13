//! Core Audio's HAL: the properties of the system object and its devices, and what its errors
//! mean to the output.

use super::super::{BackendError, ErrorKind};
use objc2_audio_toolbox::{
    kAudioComponentErr_InstanceInvalidated, kAudioUnitErr_FormatNotSupported,
    kAudioUnitErr_InvalidPropertyValue,
};
use objc2_core_audio::{
    kAudioDevicePermissionsError, kAudioDevicePropertyAvailableNominalSampleRates,
    kAudioDevicePropertyBufferFrameSize, kAudioDevicePropertyBufferFrameSizeRange,
    kAudioDevicePropertyDeviceIsAlive, kAudioDevicePropertyDeviceUID, kAudioDevicePropertyHogMode,
    kAudioDevicePropertyLatency, kAudioDevicePropertyNominalSampleRate,
    kAudioDevicePropertySafetyOffset, kAudioDevicePropertyStreams,
    kAudioDeviceUnsupportedFormatError, kAudioHardwareBadDeviceError, kAudioHardwareBadObjectError,
    kAudioHardwareBadPropertySizeError, kAudioHardwareBadStreamError, kAudioHardwareNotReadyError,
    kAudioHardwareNotRunningError, kAudioHardwarePropertyDefaultOutputDevice,
    kAudioHardwarePropertyDevices, kAudioHardwarePropertyTranslateUIDToDevice,
    kAudioHardwareUnknownPropertyError, kAudioHardwareUnsupportedOperationError,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyName, kAudioObjectPropertyScopeGlobal,
    kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject, kAudioObjectUnknown,
    kAudioStreamPropertyLatency, AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize,
    AudioObjectID, AudioObjectPropertyAddress, AudioObjectPropertyScope,
    AudioObjectPropertySelector, AudioObjectSetPropertyData,
};
use objc2_core_audio_types::{AudioStreamBasicDescription, AudioValueRange};
use objc2_core_foundation::{CFRetained, CFString};
use std::ffi::c_void;
use std::mem::{size_of, MaybeUninit};
use std::ptr::{null, NonNull};

pub(super) type OSStatus = i32;
pub(super) type DeviceId = AudioObjectID;

/// What device ids start with: what cpal's Core Audio host named devices with, so a device saved
/// before Core Audio was used directly is found again.
const ID_PREFIX: &str = "coreaudio:";

pub(super) const SYSTEM: AudioObjectID = kAudioObjectSystemObject as AudioObjectID;
pub(super) const GLOBAL: AudioObjectPropertyScope = kAudioObjectPropertyScopeGlobal;
pub(super) const OUTPUT: AudioObjectPropertyScope = kAudioObjectPropertyScopeOutput;

/// The [`crate::settings::OutputDevice::id`] of the device with `uid`.
pub(super) fn device_key(uid: &str) -> String {
    format!("{ID_PREFIX}{uid}")
}

/// The UID in an [`crate::settings::OutputDevice::id`], `None` for another platform's.
pub(super) fn uid_of(key: &str) -> Option<&str> {
    key.strip_prefix(ID_PREFIX)
}

/// The output's error for `status`, returned by `what`.
pub(super) fn error(what: &str, status: OSStatus) -> BackendError {
    BackendError::new(
        kind(status),
        format!("Core Audio: {what} failed: {}", describe(status)),
    )
}

/// Turns `status` into a result, with [`error`] for a failure.
pub(super) fn check(status: OSStatus, what: &str) -> Result<(), BackendError> {
    if status == 0 {
        Ok(())
    } else {
        Err(error(what, status))
    }
}

/// What a failure means to the output: whether opening again may help.
pub(super) fn kind(status: OSStatus) -> ErrorKind {
    #[allow(non_upper_case_globals)]
    match status {
        // The device went away under the stream, or the unit with it.
        kAudioHardwareBadDeviceError
        | kAudioHardwareBadObjectError
        | kAudioHardwareBadStreamError
        | kAudioComponentErr_InstanceInvalidated => ErrorKind::Invalidated,
        // Another process holds the device in hog mode.
        kAudioDevicePermissionsError => ErrorKind::Busy,
        kAudioDeviceUnsupportedFormatError
        | kAudioUnitErr_FormatNotSupported
        | kAudioUnitErr_InvalidPropertyValue
        | kAudioHardwareUnsupportedOperationError
        | kAudioHardwareUnknownPropertyError
        | kAudioHardwareBadPropertySizeError => ErrorKind::Unsupported,
        // coreaudiod is not running, or restarting.
        kAudioHardwareNotRunningError | kAudioHardwareNotReadyError => ErrorKind::Unavailable,
        // Anything else may pass, like 'nope' while the device reconfigures.
        _ => ErrorKind::Backend,
    }
}

/// `status` as Core Audio's four characters where it is one, like `'!dev'`, and as a number.
pub(super) fn describe(status: OSStatus) -> String {
    let bytes = status.to_be_bytes();
    if bytes
        .iter()
        .all(|&byte| byte.is_ascii_graphic() || byte == b' ')
    {
        let code: String = bytes.iter().map(|&byte| char::from(byte)).collect();
        format!("'{code}' ({status})")
    } else {
        status.to_string()
    }
}

/// A value Core Audio hands over as plain bytes, valid when zeroed.
///
/// # Safety
///
/// Every bit pattern, and all zeroes, must be a valid value.
pub(super) unsafe trait Plain: Copy {}

// SAFETY: plain numbers and structs of them.
unsafe impl Plain for u32 {}
unsafe impl Plain for i32 {}
unsafe impl Plain for f64 {}
unsafe impl Plain for AudioValueRange {}
unsafe impl Plain for AudioStreamBasicDescription {}

fn address(
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: kAudioObjectPropertyElementMain,
    }
}

/// The property `selector` of `object`.
pub(super) fn get<T: Plain>(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Result<T, OSStatus> {
    let address = address(selector, scope);
    let mut value = MaybeUninit::<T>::zeroed();
    let mut size = size_of::<T>() as u32;
    // SAFETY: `value` has room for `size` bytes, which is all the HAL writes.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut size),
            NonNull::from(&mut value).cast(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    if size as usize != size_of::<T>() {
        return Err(kAudioHardwareBadPropertySizeError);
    }
    // SAFETY: zeroed, then written by the HAL: valid either way for a `Plain` type.
    Ok(unsafe { value.assume_init() })
}

/// The property `selector` of `object`, a list.
pub(super) fn get_list<T: Plain>(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Result<Vec<T>, OSStatus> {
    let address = address(selector, scope);
    let mut size = 0u32;
    // SAFETY: writes the size into `size`.
    let status = unsafe {
        AudioObjectGetPropertyDataSize(
            object,
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut size),
        )
    };
    if status != 0 {
        return Err(status);
    }
    let count = size as usize / size_of::<T>();
    if count == 0 {
        return Ok(vec![]);
    }
    // SAFETY: all zeroes is a valid `T`.
    let mut values = vec![unsafe { MaybeUninit::<T>::zeroed().assume_init() }; count];
    let mut size = (count * size_of::<T>()) as u32;
    // SAFETY: `values` has room for `size` bytes, which is all the HAL writes.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut size),
            NonNull::new_unchecked(values.as_mut_ptr()).cast(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    values.truncate(size as usize / size_of::<T>());
    Ok(values)
}

/// Sets the property `selector` of `object` to `value`.
pub(super) fn set<T: Plain>(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
    value: T,
) -> Result<(), OSStatus> {
    let address = address(selector, scope);
    // SAFETY: the HAL reads `size_of::<T>()` bytes of `value`.
    let status = unsafe {
        AudioObjectSetPropertyData(
            object,
            NonNull::from(&address),
            0,
            null(),
            size_of::<T>() as u32,
            NonNull::from(&value).cast(),
        )
    };
    if status == 0 {
        Ok(())
    } else {
        Err(status)
    }
}

/// The string property `selector` of `object`.
fn string(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Option<String> {
    let address = address(selector, scope);
    let mut value: *const CFString = null();
    let mut size = size_of::<*const CFString>() as u32;
    // SAFETY: the HAL writes a CFString reference into `value`.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&address),
            0,
            null(),
            NonNull::from(&mut size),
            NonNull::from(&mut value).cast(),
        )
    };
    if status != 0 {
        return None;
    }
    let value = NonNull::new(value.cast_mut())?;
    // SAFETY: string properties follow the create rule: the reference is ours to release.
    let value = unsafe { CFRetained::from_raw(value) };
    Some(value.to_string())
}

/// Every device there is.
pub(super) fn devices() -> Vec<DeviceId> {
    get_list(SYSTEM, kAudioHardwarePropertyDevices, GLOBAL).unwrap_or_else(|status| {
        log::warn!("Could not list the output devices: {}", describe(status));
        vec![]
    })
}

/// The system's default output device.
pub(super) fn default_output() -> Option<DeviceId> {
    get::<u32>(SYSTEM, kAudioHardwarePropertyDefaultOutputDevice, GLOBAL)
        .ok()
        .filter(|&device| device != kAudioObjectUnknown)
}

pub(super) fn uid(device: DeviceId) -> Option<String> {
    string(device, kAudioDevicePropertyDeviceUID, GLOBAL)
}

pub(super) fn name(device: DeviceId) -> Option<String> {
    string(device, kAudioObjectPropertyName, GLOBAL)
}

/// The device has output streams, and so plays.
pub(super) fn has_output(device: DeviceId) -> bool {
    get_list::<u32>(device, kAudioDevicePropertyStreams, OUTPUT)
        .is_ok_and(|streams| !streams.is_empty())
}

/// The output device with `uid`, while it is there.
pub(super) fn by_uid(uid: &str) -> Option<DeviceId> {
    let string = CFString::from_str(uid);
    let reference: *const CFString = CFRetained::as_ptr(&string).as_ptr();
    let address = address(kAudioHardwarePropertyTranslateUIDToDevice, GLOBAL);
    let mut device: AudioObjectID = kAudioObjectUnknown;
    let mut size = size_of::<AudioObjectID>() as u32;
    // SAFETY: the qualifier is the CFString reference, which outlives the call; the HAL writes
    // a device id into `device`.
    let status = unsafe {
        AudioObjectGetPropertyData(
            SYSTEM,
            NonNull::from(&address),
            size_of::<*const CFString>() as u32,
            (&reference as *const *const CFString).cast::<c_void>(),
            NonNull::from(&mut size),
            NonNull::from(&mut device).cast(),
        )
    };
    (status == 0 && device != kAudioObjectUnknown && has_output(device)).then_some(device)
}

/// The device to play on: the one with `chosen` UID while it is there, else the default.
pub(super) fn resolve(chosen: Option<&str>) -> Option<DeviceId> {
    chosen.and_then(by_uid).or_else(default_output)
}

pub(super) fn is_alive(device: DeviceId) -> bool {
    get::<u32>(device, kAudioDevicePropertyDeviceIsAlive, GLOBAL).is_ok_and(|alive| alive != 0)
}

/// The rate the device runs at.
pub(super) fn nominal_rate(device: DeviceId) -> Result<u32, OSStatus> {
    get::<f64>(device, kAudioDevicePropertyNominalSampleRate, GLOBAL)
        .map(|rate| rate.round() as u32)
}

pub(super) fn set_nominal_rate(device: DeviceId, rate: u32) -> Result<(), OSStatus> {
    set(
        device,
        kAudioDevicePropertyNominalSampleRate,
        GLOBAL,
        f64::from(rate),
    )
}

/// The rates the device can run at, as ranges; a single rate has its minimum and maximum equal.
pub(super) fn available_rates(device: DeviceId) -> Vec<(f64, f64)> {
    get_list::<AudioValueRange>(
        device,
        kAudioDevicePropertyAvailableNominalSampleRates,
        GLOBAL,
    )
    .unwrap_or_default()
    .into_iter()
    .map(|range| (range.mMinimum, range.mMaximum))
    .collect()
}

/// The process that holds the device in hog mode, if one does.
pub(super) fn hog_owner(device: DeviceId) -> Option<i32> {
    get::<i32>(device, kAudioDevicePropertyHogMode, GLOBAL)
        .ok()
        .filter(|&pid| pid != -1)
}

/// The range of I/O buffer sizes the device takes, in frames.
pub(super) fn buffer_frame_range(device: DeviceId) -> Option<(u32, u32)> {
    get::<AudioValueRange>(device, kAudioDevicePropertyBufferFrameSizeRange, GLOBAL)
        .ok()
        .map(|range| {
            (
                range.mMinimum.max(1.0) as u32,
                range.mMaximum.max(1.0) as u32,
            )
        })
}

pub(super) fn buffer_frame_size(device: DeviceId) -> Option<u32> {
    get::<u32>(device, kAudioDevicePropertyBufferFrameSize, GLOBAL).ok()
}

pub(super) fn set_buffer_frame_size(device: DeviceId, frames: u32) -> Result<(), OSStatus> {
    set(device, kAudioDevicePropertyBufferFrameSize, GLOBAL, frames)
}

/// The output latencies of the device, in frames: its own, its safety offset and its first
/// output stream's.
pub(super) fn latencies(device: DeviceId) -> (u32, u32, u32) {
    let latency = get::<u32>(device, kAudioDevicePropertyLatency, OUTPUT).unwrap_or(0);
    let safety = get::<u32>(device, kAudioDevicePropertySafetyOffset, OUTPUT).unwrap_or(0);
    let stream = get_list::<u32>(device, kAudioDevicePropertyStreams, OUTPUT)
        .ok()
        .and_then(|streams| streams.first().copied())
        .and_then(|stream| get::<u32>(stream, kAudioStreamPropertyLatency, GLOBAL).ok())
        .unwrap_or(0);
    (latency, safety, stream)
}
