//! The state a playback task shares with the queue side.

use super::Next;
use crate::TrackTime;
use std::sync::{Arc, Mutex};
use std::thread::Thread;
use std::time::{Duration, Instant};
use symphonia::core::units::Time;

struct State {
    volume: f32,
    /// The device chosen to play on.
    device: Option<String>,
    paused: bool,
    reload_output: bool,
    seek: Option<(Time, u64)>,
    revision: u64,
    version: u64,
    stopped: bool,
    running: bool,
    finished: bool,
    /// The last measured position, its seek revision and, while audio advances, when it was
    /// measured.
    time: Option<(TrackTime, u64, Option<Instant>)>,
    progress_interval: Option<Duration>,
    next: Option<Next>,
    /// How long the track fades into the next; zero hands over without a gap.
    crossfade: Duration,
    /// The task's thread, woken on every change.
    thread: Option<Thread>,
}

/// The queue side's handle on a running task; every change wakes the task's thread.
#[derive(Clone)]
pub(super) struct PlaybackControl(Arc<Mutex<State>>);

/// A snapshot of the controls, taken once per round of the playback loop.
pub(super) struct Controls {
    pub(super) volume: f32,
    pub(super) paused: bool,
    pub(super) reload_output: bool,
    pub(super) seek: Option<(Time, u64)>,
    pub(super) version: u64,
    pub(super) stopped: bool,
    pub(super) progress_interval: Option<Duration>,
    pub(super) next: Option<Next>,
    pub(super) crossfade: Duration,
}

impl PlaybackControl {
    pub(super) fn new(volume: f32, device: Option<String>) -> Self {
        Self(Arc::new(Mutex::new(State {
            volume,
            device,
            paused: false,
            reload_output: false,
            seek: None,
            revision: 0,
            version: 0,
            stopped: false,
            running: false,
            finished: false,
            time: None,
            progress_interval: None,
            next: None,
            crossfade: Duration::ZERO,
            thread: None,
        })))
    }

    fn update(&self, change: impl FnOnce(&mut State)) {
        let mut state = self.0.lock().unwrap();
        change(&mut state);
        state.version = state.version.wrapping_add(1);
        if let Some(thread) = &state.thread {
            thread.unpark();
        }
    }

    pub(super) fn set_paused(&self, paused: bool) {
        self.update(|state| state.paused = paused);
    }

    pub(super) fn reload_output(&self) {
        self.update(|state| state.reload_output = true);
    }

    pub(super) fn set_output_device(&self, device: Option<String>) {
        self.update(|state| state.device = device);
    }

    /// The device chosen to play on.
    pub(super) fn output_device(&self) -> Option<String> {
        self.0.lock().unwrap().device.clone()
    }

    pub(super) fn set_volume(&self, volume: f32) {
        self.update(|state| state.volume = volume.clamp(0.0, 1.0));
    }

    pub(super) fn set_progress_interval(&self, interval: Option<Duration>) {
        self.update(|state| state.progress_interval = interval);
    }

    pub(super) fn set_next(&self, next: Option<Next>) {
        self.update(|state| state.next = next);
    }

    pub(super) fn set_crossfade(&self, crossfade: Duration) {
        self.update(|state| state.crossfade = crossfade);
    }

    pub(super) fn seek(&self, seconds: f64) -> u64 {
        let seconds = seconds.max(0.0);
        let mut revision = 0;
        self.update(|state| {
            state.revision = state.revision.wrapping_add(1);
            revision = state.revision;
            state.seek = Some((
                Time::try_from_secs_f64(seconds).unwrap_or_default(),
                revision,
            ));
        });
        revision
    }

    pub(super) fn revision(&self) -> u64 {
        self.0.lock().unwrap().revision
    }

    /// Where playback is now: the task measures it only when it wakes up, so the time since
    /// is added while audio advances.
    pub(super) fn time(&self) -> Option<TrackTime> {
        let state = self.0.lock().unwrap();
        let (mut time, revision, measured) = state.time?;
        if revision != state.revision {
            return None;
        }
        if let Some(measured) = measured {
            time.position += measured.elapsed().as_secs_f64();
            if time.length > 0.0 {
                time.position = time.position.min(time.length);
            }
        }
        Some(time)
    }

    pub(super) fn is_paused(&self) -> bool {
        self.0.lock().unwrap().paused
    }

    pub(super) fn is_playing(&self) -> bool {
        let state = self.0.lock().unwrap();
        !state.stopped && !state.finished
    }

    pub(super) fn stop(&self) {
        self.update(|state| {
            state.stopped = true;
            if !state.running {
                state.finished = true;
            }
        });
    }

    /// Registers the calling thread as the task's; false if it was stopped already.
    pub(super) fn begin(&self) -> bool {
        let mut state = self.0.lock().unwrap();
        if state.stopped {
            return false;
        }
        state.running = true;
        state.thread = Some(std::thread::current());
        true
    }

    pub(super) fn finish(&self) {
        let mut state = self.0.lock().unwrap();
        state.finished = true;
        state.running = false;
        state.thread = None;
    }

    pub(super) fn controls(&self) -> Controls {
        let mut state = self.0.lock().unwrap();
        Controls {
            volume: state.volume,
            paused: state.paused,
            reload_output: std::mem::take(&mut state.reload_output),
            seek: state.seek.take(),
            version: state.version,
            stopped: state.stopped,
            progress_interval: state.progress_interval,
            next: state.next.clone(),
            crossfade: state.crossfade,
        }
    }

    /// Publishes the position; `advancing` while audio is playing on from it.
    pub(super) fn publish(&self, time: TrackTime, revision: u64, advancing: bool) {
        let measured = advancing.then(Instant::now);
        self.0.lock().unwrap().time = Some((time, revision, measured));
    }
}

/// Marks the task finished however it exits.
pub(super) struct Completion(pub(super) PlaybackControl);

impl Drop for Completion {
    fn drop(&mut self) {
        self.0.finish();
    }
}
