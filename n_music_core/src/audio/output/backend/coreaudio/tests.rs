use super::*;
use crate::audio::output::callback::SharedState;
use objc2_audio_toolbox::{
    kAudioComponentErr_InstanceInvalidated, kAudioUnitErr_FormatNotSupported,
};
use objc2_core_audio::{
    kAudioDevicePermissionsError, kAudioDeviceUnsupportedFormatError, kAudioHardwareBadDeviceError,
    kAudioHardwareIllegalOperationError, kAudioHardwareNotRunningError,
};
use rtrb::RingBuffer;
use std::sync::Arc;

#[test]
fn maps_core_audio_errors_to_what_they_mean_to_the_output() {
    assert_eq!(
        hal::kind(kAudioHardwareBadDeviceError),
        ErrorKind::Invalidated
    );
    assert_eq!(
        hal::kind(kAudioComponentErr_InstanceInvalidated),
        ErrorKind::Invalidated
    );
    assert_eq!(hal::kind(kAudioDevicePermissionsError), ErrorKind::Busy);
    assert_eq!(
        hal::kind(kAudioDeviceUnsupportedFormatError),
        ErrorKind::Unsupported
    );
    assert_eq!(
        hal::kind(kAudioUnitErr_FormatNotSupported),
        ErrorKind::Unsupported
    );
    assert_eq!(
        hal::kind(kAudioHardwareNotRunningError),
        ErrorKind::Unavailable
    );
    assert_eq!(
        hal::kind(kAudioHardwareIllegalOperationError),
        ErrorKind::Backend
    );
    assert_eq!(hal::kind(-1), ErrorKind::Backend);
    assert!(hal::error("x", kAudioHardwareBadDeviceError).is_recoverable());
    assert!(hal::error("x", kAudioDevicePermissionsError).is_recoverable());
    assert!(!hal::error("x", kAudioDeviceUnsupportedFormatError).is_recoverable());
}

#[test]
fn describes_statuses_as_their_four_characters() {
    assert_eq!(
        hal::describe(kAudioHardwareBadDeviceError),
        "'!dev' (560227702)"
    );
    assert_eq!(
        hal::describe(kAudioDevicePermissionsError),
        "'!hog' (560492391)"
    );
    assert_eq!(hal::describe(-10868), "-10868");
    assert_eq!(
        hal::error("starting the output", kAudioHardwareBadDeviceError).to_string(),
        "Core Audio: starting the output failed: '!dev' (560227702)"
    );
}

#[test]
fn device_ids_are_the_ones_cpal_saved() {
    assert_eq!(
        hal::device_key("BuiltInSpeakerDevice"),
        "coreaudio:BuiltInSpeakerDevice"
    );
    assert_eq!(
        hal::uid_of("coreaudio:AppleUSBAudioEngine:Vendor:DAC:1234:1"),
        Some("AppleUSBAudioEngine:Vendor:DAC:1234:1")
    );
    assert_eq!(hal::uid_of("pulseaudio:sink"), None);
}

#[test]
fn renders_silence_when_the_renderer_panics() {
    let mut samples = [1.0f32; 8];
    let mut list = AudioBufferList {
        mNumberBuffers: 1,
        mBuffers: [objc2_core_audio_types::AudioBuffer {
            mNumberChannels: 2,
            mDataByteSize: std::mem::size_of_val(&samples) as u32,
            mData: samples.as_mut_ptr().cast(),
        }],
    };
    // A renderer for no channels divides by zero, a stand-in for any panic.
    let (_, consumer) = RingBuffer::<f32>::new(8);
    let state = Arc::new(SharedState::new(1.0));
    let format = OutputFormat {
        rate: 48_000,
        channels: 0,
    };
    let mut callback = Callback {
        renderer: Renderer::new(consumer, state, format, 4),
        channels: 2,
        latency: Duration::ZERO,
    };
    let mut timestamp: AudioTimeStamp = unsafe { std::mem::zeroed() };
    timestamp.mFlags = AudioTimeStampFlags::NothingValid;
    let mut flags = AudioUnitRenderActionFlags(0);
    // SAFETY: a valid list and callback.
    let status = unsafe {
        render(
            NonNull::from(&mut callback).cast(),
            NonNull::from(&mut flags),
            NonNull::from(&timestamp),
            0,
            4,
            &mut list,
        )
    };
    assert_eq!(status, 0);
    assert!(samples.iter().all(|&sample| sample == 0.0), "{samples:?}");
}

/// Opens the default device for audio at 44.1, 48 and 96 kHz, checks that it runs at each one it
/// has, then opens it without a rate to match and checks that it is back at its own rate.
#[test]
#[ignore = "switches the rate of the real default output device"]
fn switches_the_default_device_to_each_rate_it_has() {
    let device = hal::default_output().expect("a default output device");
    let original = hal::nominal_rate(device).expect("its rate");
    let available = hal::available_rates(device);
    for wanted in [44_100, 48_000, 96_000] {
        let state = Arc::new(SharedState::new(0.0));
        let request = Request {
            device: None,
            offload: false,
            rate: Some(wanted),
            state: state.clone(),
        };
        let expected = rate::pick(wanted, &available)
            .unwrap_or_else(|| hal::nominal_rate(device).expect("its rate"));
        assert_eq!(CoreAudio::rate_for(None, wanted), Some(expected));
        let mut opened = CoreAudio::open(&request, |shape| {
            let (_, consumer) = RingBuffer::new(shape.format.rate as usize);
            Renderer::new(
                consumer,
                state.clone(),
                shape.format,
                shape.format.rate as usize,
            )
        })
        .expect("the default device opens");
        assert_eq!(opened.shape.format.rate, expected);
        opened.stream.play().expect("it plays");
        std::thread::sleep(Duration::from_millis(200));
        assert!(
            state
                .heartbeat_ns
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0,
            "the render callback runs"
        );
        opened.stream.pause().expect("it pauses");
        assert!(!opened.stream.moved());
    }
    // As once rate matching is turned off.
    let state = Arc::new(SharedState::new(0.0));
    let request = Request {
        device: None,
        offload: false,
        rate: None,
        state: state.clone(),
    };
    let restored = CoreAudio::open(&request, |shape| {
        let (_, consumer) = RingBuffer::new(shape.format.rate as usize);
        Renderer::new(
            consumer,
            state.clone(),
            shape.format,
            shape.format.rate as usize,
        )
    })
    .map(|opened| opened.shape.format.rate);
    let now = hal::nominal_rate(device).expect("its rate");
    if now != original {
        // Never leave the device at another rate, whatever failed.
        let _ = rate::switch(device, now, original);
    }
    assert_eq!(restored.expect("the default device opens"), original);
    assert_eq!(now, original, "opening without a rate puts the device back");
}
