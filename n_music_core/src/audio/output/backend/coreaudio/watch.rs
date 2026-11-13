//! Follows the device a stream plays on through the HAL's property listeners, without polling:
//! the stream is opened again once it should be on another device, its device went away, or
//! coreaudiod restarted.

use super::hal::{self, DeviceId, GLOBAL, SYSTEM};
use crate::audio::output::callback::SharedState;
use objc2_core_audio::{
    kAudioDeviceProcessorOverload, kAudioDevicePropertyDeviceIsAlive,
    kAudioHardwarePropertyDefaultOutputDevice, kAudioHardwarePropertyDevices,
    kAudioHardwarePropertyServiceRestarted, kAudioObjectPropertyElementMain,
    AudioObjectAddPropertyListener, AudioObjectID, AudioObjectPropertyAddress,
    AudioObjectPropertySelector, AudioObjectRemovePropertyListener,
};
use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Processor overloads are logged at most this often.
const OVERLOAD_LOG_INTERVAL_NS: u64 = 5_000_000_000;

/// What the listeners see; shared with the HAL's notification thread.
struct Context {
    /// The device the stream plays on.
    device: DeviceId,
    /// The UID of the device asked for, if one was.
    chosen: Option<String>,
    moved: AtomicBool,
    state: Arc<SharedState>,
    origin: Instant,
    /// When an overload was last logged, from `origin`; 0 before.
    overload_logged_ns: AtomicU64,
}

impl Context {
    fn changed(&self, selector: AudioObjectPropertySelector) {
        #[allow(non_upper_case_globals)]
        let moved = match selector {
            kAudioDeviceProcessorOverload => {
                self.overloaded();
                false
            }
            kAudioDevicePropertyDeviceIsAlive => !hal::is_alive(self.device),
            kAudioHardwarePropertyServiceRestarted => true,
            kAudioHardwarePropertyDefaultOutputDevice | kAudioHardwarePropertyDevices => {
                hal::resolve(self.chosen.as_deref()) != Some(self.device)
            }
            _ => false,
        };
        if moved && !self.moved.swap(true, Ordering::Relaxed) {
            log::info!(
                "The output should move ({}): opening it again",
                hal::describe(selector as i32)
            );
            // Wakes the writer, which finds the stream moved.
            self.state.glitch();
        }
    }

    fn overloaded(&self) {
        let now = (self.origin.elapsed().as_nanos() as u64).max(1);
        let last = self.overload_logged_ns.load(Ordering::Relaxed);
        if last == 0 || now.saturating_sub(last) >= OVERLOAD_LOG_INTERVAL_NS {
            self.overload_logged_ns.store(now, Ordering::Relaxed);
            log::warn!(
                "The output device reported a processor overload: audio may have dropped out"
            );
        }
    }
}

unsafe extern "C-unwind" fn listener(
    _object: AudioObjectID,
    count: u32,
    addresses: NonNull<AudioObjectPropertyAddress>,
    data: *mut c_void,
) -> i32 {
    // Never unwind into the HAL.
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `data` is the `Context` the listener was added with, alive until it is removed;
        // the HAL hands over `count` addresses.
        let (context, addresses) = unsafe {
            (
                &*data.cast::<Context>(),
                std::slice::from_raw_parts(addresses.as_ptr(), count as usize),
            )
        };
        for address in addresses {
            context.changed(address.mSelector);
        }
    }));
    0
}

/// The listeners of one stream, removed when it is dropped.
pub(super) struct Watch {
    context: NonNull<Context>,
    added: Vec<(AudioObjectID, AudioObjectPropertyAddress)>,
}

// SAFETY: the context is only read through shared references, and its fields are thread safe.
unsafe impl Send for Watch {}

impl Watch {
    /// Watches a stream on `device`, `chosen` being the UID of the device asked for. Without one,
    /// or while it is not there, the stream follows the default device; with one, it goes back
    /// to it once it is there again.
    pub(super) fn new(device: DeviceId, chosen: Option<String>, state: Arc<SharedState>) -> Self {
        let on_chosen = chosen.as_deref().and_then(hal::by_uid) == Some(device);
        let watches_devices = chosen.is_some();
        let context = Box::new(Context {
            device,
            chosen,
            moved: AtomicBool::new(false),
            state,
            origin: Instant::now(),
            overload_logged_ns: AtomicU64::new(0),
        });
        let mut watch = Self {
            context: NonNull::from(Box::leak(context)),
            added: vec![],
        };
        watch.add(device, kAudioDevicePropertyDeviceIsAlive);
        watch.add(device, kAudioDeviceProcessorOverload);
        watch.add(SYSTEM, kAudioHardwarePropertyServiceRestarted);
        if !on_chosen {
            watch.add(SYSTEM, kAudioHardwarePropertyDefaultOutputDevice);
        }
        if watches_devices {
            watch.add(SYSTEM, kAudioHardwarePropertyDevices);
        }
        // The device may have changed between opening the stream and listening.
        if !hal::is_alive(device) {
            watch.context().changed(kAudioDevicePropertyDeviceIsAlive);
        } else {
            watch.context().changed(kAudioHardwarePropertyDevices);
        }
        watch
    }

    fn context(&self) -> &Context {
        // SAFETY: owned by the watch until it is dropped.
        unsafe { self.context.as_ref() }
    }

    fn add(&mut self, object: AudioObjectID, selector: AudioObjectPropertySelector) {
        let address = AudioObjectPropertyAddress {
            mSelector: selector,
            mScope: GLOBAL,
            mElement: kAudioObjectPropertyElementMain,
        };
        // SAFETY: the context outlives the listener, which is removed before it is freed.
        let status = unsafe {
            AudioObjectAddPropertyListener(
                object,
                NonNull::from(&address),
                Some(listener),
                self.context.as_ptr().cast(),
            )
        };
        if status == 0 {
            self.added.push((object, address));
        } else {
            log::warn!(
                "Could not listen to {} of the output: {}",
                hal::describe(selector as i32),
                hal::describe(status)
            );
        }
    }

    /// The stream should be opened again.
    pub(super) fn moved(&self) -> bool {
        self.context().moved.load(Ordering::Relaxed)
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        for (object, address) in self.added.drain(..) {
            // SAFETY: removes the listener added with the same arguments. The HAL serializes
            // removal with delivery on its notification thread, so none runs past this.
            let status = unsafe {
                AudioObjectRemovePropertyListener(
                    object,
                    NonNull::from(&address),
                    Some(listener),
                    self.context.as_ptr().cast(),
                )
            };
            if status != 0 {
                log::debug!(
                    "Could not stop listening to the output: {}",
                    hal::describe(status)
                );
            }
        }
        // SAFETY: made from a box in `new`, and no listener refers to it any more.
        drop(unsafe { Box::from_raw(self.context.as_ptr()) });
    }
}
