//! Running the device at the rate of what plays, so nothing is resampled: the rate itself where
//! the device has it, else its smallest whole multiple that it has, else the rate it runs at.

use super::hal::{self, DeviceId, GLOBAL};
use objc2_core_audio::{
    kAudioDevicePropertyDeviceIsAlive, kAudioDevicePropertyNominalSampleRate,
    kAudioHardwareBadDeviceError, kAudioObjectPropertyElementMain, AudioObjectAddPropertyListener,
    AudioObjectID, AudioObjectPropertyAddress, AudioObjectRemovePropertyListener,
};
use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::ptr::NonNull;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// The multiples of a rate tried after the rate itself, smallest first.
const MULTIPLES: std::ops::RangeInclusive<u32> = 2..=8;
/// The longest wait for the device to apply another rate: USB DACs, HDMI and AirPlay receivers
/// and aggregate devices take seconds.
const SWITCH_TIMEOUT: Duration = Duration::from_secs(4);
/// How often the device is asked whether it applied the rate, should its listener say nothing.
const SWITCH_POLL: Duration = Duration::from_millis(50);

/// Rates a device (by UID) lists but did not switch to when asked: they are not asked for again,
/// so such a device keeps its rate instead of being reopened for every track. A rate is asked
/// for again once the device is seen running at it.
static REFUSED: Mutex<Vec<(String, u32)>> = Mutex::new(Vec::new());

/// The devices rate matching switched, with the rate each ran at before: switched back once an
/// output opens on one without a rate to match (see [`restore`]).
static ORIGINALS: Mutex<Vec<Original>> = Mutex::new(Vec::new());

#[derive(Clone, Debug, PartialEq, Eq)]
struct Original {
    uid: String,
    /// What the device ran at before rate matching switched it.
    rate: u32,
    /// What rate matching last asked it for: running at another, it was switched by someone
    /// else since, and is left as it is.
    asked: u32,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The rate to run a device that has the nominal rates `available` at, for audio at `wanted` Hz:
/// `wanted` itself, else its smallest whole multiple it has, else `None`.
pub(super) fn pick(wanted: u32, available: &[(f64, f64)]) -> Option<u32> {
    if wanted == 0 {
        return None;
    }
    let has = |rate: u32| {
        let rate = f64::from(rate);
        available
            .iter()
            .any(|&(min, max)| rate >= min - 0.5 && rate <= max + 0.5)
    };
    std::iter::once(1)
        .chain(MULTIPLES)
        .filter_map(|multiple| wanted.checked_mul(multiple))
        .find(|&rate| has(rate))
}

fn refused(uid: &str, rate: u32) -> bool {
    lock(&REFUSED)
        .iter()
        .any(|(device, refused)| device == uid && *refused == rate)
}

fn refuse(uid: Option<&str>, rate: u32) {
    let Some(uid) = uid else {
        return;
    };
    let mut refused = lock(&REFUSED);
    if !refused
        .iter()
        .any(|(device, other)| device == uid && *other == rate)
    {
        refused.push((uid.to_owned(), rate));
    }
}

/// The device with `uid` runs at `rate`, so it takes it after all.
fn accept(uid: &str, rate: u32) {
    lock(&REFUSED).retain(|(device, refused)| !(device == uid && *refused == rate));
}

/// Rate matching asked the device with `uid`, running at `from` Hz, for `to`: the rate it ran
/// at first is kept, unless someone else switched it since rate matching last did.
fn remember(uid: &str, from: u32, to: u32) {
    let mut originals = lock(&ORIGINALS);
    match originals.iter_mut().find(|original| original.uid == uid) {
        Some(original) => {
            if original.asked != from {
                original.rate = from;
            }
            original.asked = to;
        }
        None => originals.push(Original {
            uid: uid.to_owned(),
            rate: from,
            asked: to,
        }),
    }
}

/// What the device with `uid` ran at before rate matching switched it, which is forgotten.
fn forget(uid: &str) -> Option<Original> {
    let mut originals = lock(&ORIGINALS);
    let index = originals.iter().position(|original| original.uid == uid)?;
    Some(originals.swap_remove(index))
}

/// The device went away: `status` says so, or it is not alive any more.
fn lost(device: DeviceId, status: hal::OSStatus) -> bool {
    hal::kind(status) == super::ErrorKind::Invalidated || !hal::is_alive(device)
}

/// What `device` runs at now, and what it would run at for audio at `wanted` Hz. It stays where
/// it is when it has no fitting rate, refused this one before, or another process holds it in
/// hog mode.
pub(super) fn target(device: DeviceId, wanted: u32) -> Result<(u32, u32), hal::OSStatus> {
    let current = hal::nominal_rate(device)?;
    if !lock(&REFUSED).is_empty() {
        if let Some(uid) = hal::uid(device) {
            accept(&uid, current);
        }
    }
    if current == wanted {
        return Ok((current, current));
    }
    if let Some(owner) = hal::hog_owner(device).filter(|&pid| pid != std::process::id() as i32) {
        log::debug!("Process {owner} holds the output device: keeping it at {current} Hz");
        return Ok((current, current));
    }
    let Some(rate) = pick(wanted, &hal::available_rates(device)) else {
        return Ok((current, current));
    };
    if rate != current && hal::uid(device).is_some_and(|uid| refused(&uid, rate)) {
        return Ok((current, current));
    }
    Ok((current, rate))
}

/// How asking a device for a rate went.
enum Switched {
    /// It runs at the rate asked for.
    Done,
    /// It did not take it: it runs at this rate.
    Failed(u32),
    /// It took it but did not run at it in time: it runs at this rate for now.
    Late(u32),
}

/// Asks `device`, running at `from` Hz, for `to` and waits for it to apply. `Err` with the
/// device's status when it went away meanwhile.
fn apply(device: DeviceId, from: u32, to: u32, name: &str) -> Result<Switched, hal::OSStatus> {
    let started = Instant::now();
    if let Err(status) = hal::set_nominal_rate(device, to) {
        if lost(device, status) {
            log::warn!(
                "Lost {name} while switching it to {to} Hz: {}",
                hal::describe(status)
            );
            return Err(status);
        }
        log::warn!(
            "Could not switch {name} from {from} to {to} Hz: {}",
            hal::describe(status)
        );
        return hal::nominal_rate(device).map(Switched::Failed);
    }
    let rate = wait(device, to, SWITCH_TIMEOUT).inspect_err(|&status| {
        log::warn!(
            "Lost {name} while switching its rate: {}",
            hal::describe(status)
        );
    })?;
    if rate == to {
        log::info!(
            "Switched {name} from {from} to {to} Hz in {} ms",
            started.elapsed().as_millis()
        );
        Ok(Switched::Done)
    } else {
        Ok(Switched::Late(rate))
    }
}

/// Switches `device` from `from` Hz to `to` and waits for it to apply; returns the rate it runs
/// at in the end. A device that does not take the rate keeps the one it has, and is not asked
/// for it again until it is seen running at it. `Err` with the device's status when it went
/// away, so the output is opened again on the device there is now.
pub(super) fn switch(device: DeviceId, from: u32, to: u32) -> Result<u32, hal::OSStatus> {
    if from == to {
        return Ok(from);
    }
    let name = hal::name(device).unwrap_or_default();
    let uid = hal::uid(device);
    match apply(device, from, to, &name)? {
        Switched::Done => {
            if let Some(uid) = &uid {
                remember(uid, from, to);
                accept(uid, to);
            }
            Ok(to)
        }
        Switched::Failed(rate) => {
            refuse(uid.as_deref(), to);
            Ok(rate)
        }
        Switched::Late(rate) => {
            log::warn!(
                "{name} did not switch from {from} to {to} Hz in {} s: playing at {rate} Hz",
                SWITCH_TIMEOUT.as_secs()
            );
            if let Some(uid) = &uid {
                // It may still get there.
                remember(uid, from, to);
            }
            refuse(uid.as_deref(), to);
            Ok(rate)
        }
    }
}

/// Switches `device` back to the rate it ran at before rate matching switched it, if it did and
/// nobody switched it since, for an output that has no rate to match. `Err` with the device's
/// status when it went away.
pub(super) fn restore(device: DeviceId) -> Result<(), hal::OSStatus> {
    if lock(&ORIGINALS).is_empty() {
        return Ok(());
    }
    let Some(original) = hal::uid(device).and_then(|uid| forget(&uid)) else {
        return Ok(());
    };
    let current = hal::nominal_rate(device)?;
    let name = hal::name(device).unwrap_or_default();
    if current == original.rate {
        return Ok(());
    }
    if current != original.asked {
        log::info!(
            "{name} was switched to {current} Hz since rate matching asked for {} Hz: keeping it",
            original.asked
        );
        return Ok(());
    }
    match apply(device, current, original.rate, &name)? {
        Switched::Done => log::info!("Put {name} back to {} Hz", original.rate),
        Switched::Failed(rate) | Switched::Late(rate) => log::warn!(
            "Could not put {name} back to {} Hz: it runs at {rate} Hz",
            original.rate
        ),
    }
    Ok(())
}

/// Waits up to `timeout` for `device` to run at `to` Hz, woken by its listeners; returns the rate
/// it runs at in the end. `Err` when it went away.
fn wait(device: DeviceId, to: u32, timeout: Duration) -> Result<u32, hal::OSStatus> {
    let started = Instant::now();
    let waker = Waker::listen(device);
    loop {
        let now = hal::nominal_rate(device)?;
        if now == to {
            return Ok(now);
        }
        if !hal::is_alive(device) {
            return Err(kAudioHardwareBadDeviceError);
        }
        let left = timeout.saturating_sub(started.elapsed());
        if left.is_zero() {
            return Ok(now);
        }
        waker.wait(left.min(SWITCH_POLL));
    }
}

/// Woken when the device's rate changes or it goes away. Removes its listeners when dropped.
struct Waker {
    signal: NonNull<Signal>,
    added: Vec<(AudioObjectID, AudioObjectPropertyAddress)>,
}

#[derive(Default)]
struct Signal {
    changed: Mutex<bool>,
    condvar: Condvar,
}

unsafe extern "C-unwind" fn woken(
    _object: AudioObjectID,
    _count: u32,
    _addresses: NonNull<AudioObjectPropertyAddress>,
    data: *mut c_void,
) -> i32 {
    // Never unwind into the HAL.
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `data` is the signal the listener was added with, alive until it is removed.
        let signal = unsafe { &*data.cast::<Signal>() };
        *lock(&signal.changed) = true;
        signal.condvar.notify_all();
    }));
    0
}

impl Waker {
    fn listen(device: DeviceId) -> Self {
        let mut waker = Self {
            signal: NonNull::from(Box::leak(Box::<Signal>::default())),
            added: vec![],
        };
        for selector in [
            kAudioDevicePropertyNominalSampleRate,
            kAudioDevicePropertyDeviceIsAlive,
        ] {
            let address = AudioObjectPropertyAddress {
                mSelector: selector,
                mScope: GLOBAL,
                mElement: kAudioObjectPropertyElementMain,
            };
            // SAFETY: the signal outlives the listener, which is removed before it is freed.
            let status = unsafe {
                AudioObjectAddPropertyListener(
                    device,
                    NonNull::from(&address),
                    Some(woken),
                    waker.signal.as_ptr().cast(),
                )
            };
            if status == 0 {
                waker.added.push((device, address));
            } else {
                // The wait polls anyway.
                log::debug!(
                    "Could not listen to {} of the output: {}",
                    hal::describe(selector as i32),
                    hal::describe(status)
                );
            }
        }
        waker
    }

    /// Returns once a listener fired, or after `timeout`.
    fn wait(&self, timeout: Duration) {
        // SAFETY: owned by the waker until it is dropped.
        let signal = unsafe { self.signal.as_ref() };
        let changed = lock(&signal.changed);
        let mut changed = if *changed {
            changed
        } else {
            signal
                .condvar
                .wait_timeout(changed, timeout)
                .map(|(changed, _)| changed)
                .unwrap_or_else(|poisoned| poisoned.into_inner().0)
        };
        *changed = false;
    }
}

impl Drop for Waker {
    fn drop(&mut self) {
        for (object, address) in self.added.drain(..) {
            // SAFETY: removes the listener added with the same arguments. The HAL serializes
            // removal with delivery on its notification thread, so none runs past this.
            unsafe {
                AudioObjectRemovePropertyListener(
                    object,
                    NonNull::from(&address),
                    Some(woken),
                    self.signal.as_ptr().cast(),
                );
            }
        }
        // SAFETY: made from a box in `listen`, and no listener refers to it any more.
        drop(unsafe { Box::from_raw(self.signal.as_ptr()) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILT_IN: &[(f64, f64)] = &[
        (44_100.0, 44_100.0),
        (48_000.0, 48_000.0),
        (96_000.0, 96_000.0),
    ];

    #[test]
    fn picks_the_rate_itself_where_the_device_has_it() {
        assert_eq!(pick(44_100, BUILT_IN), Some(44_100));
        assert_eq!(pick(96_000, BUILT_IN), Some(96_000));
    }

    #[test]
    fn picks_the_smallest_multiple_the_device_has() {
        assert_eq!(pick(24_000, BUILT_IN), Some(48_000));
        assert_eq!(pick(22_050, BUILT_IN), Some(44_100));
        assert_eq!(pick(12_000, BUILT_IN), Some(48_000));
        // x8 is the last multiple tried.
        assert_eq!(pick(6_000, &[(48_000.0, 48_000.0)]), Some(48_000));
        assert_eq!(pick(4_000, &[(48_000.0, 48_000.0)]), None);
    }

    #[test]
    fn keeps_the_rate_when_nothing_fits() {
        assert_eq!(pick(88_200, BUILT_IN), None);
        assert_eq!(pick(44_100, &[(48_000.0, 48_000.0)]), None);
        assert_eq!(pick(44_100, &[]), None);
        assert_eq!(pick(0, BUILT_IN), None);
    }

    #[test]
    fn takes_any_rate_in_a_continuous_range() {
        assert_eq!(pick(44_100, &[(8_000.0, 192_000.0)]), Some(44_100));
        assert_eq!(pick(4_000, &[(8_000.0, 192_000.0)]), Some(8_000));
        assert_eq!(pick(u32::MAX, &[(8_000.0, 192_000.0)]), None);
    }

    #[test]
    fn remembers_refused_rates_per_device() {
        refuse(Some("test-device"), 88_200);
        refuse(Some("test-device"), 88_200);
        refuse(Some("test-device"), 176_400);
        assert!(refused("test-device", 88_200));
        assert!(!refused("test-device", 96_000));
        assert!(!refused("other-device", 88_200));
        // Seen running at it, as a device that took the rate after the wait.
        accept("test-device", 88_200);
        assert!(!refused("test-device", 88_200));
        assert!(refused("test-device", 176_400));
    }

    #[test]
    fn remembers_the_rate_a_device_ran_at_before_matching() {
        remember("matched-device", 48_000, 44_100);
        remember("matched-device", 44_100, 96_000);
        assert_eq!(
            forget("matched-device"),
            Some(Original {
                uid: "matched-device".into(),
                rate: 48_000,
                asked: 96_000,
            }),
            "the first rate is the one put back"
        );
        assert_eq!(forget("matched-device"), None, "and it is forgotten");
    }

    #[test]
    fn takes_a_rate_set_by_someone_else_as_the_original() {
        remember("shared-device", 48_000, 44_100);
        // The user set 88.2 kHz in Audio MIDI Setup, then a 96 kHz track played.
        remember("shared-device", 88_200, 96_000);
        assert_eq!(
            forget("shared-device").map(|original| (original.rate, original.asked)),
            Some((88_200, 96_000))
        );
    }
}
