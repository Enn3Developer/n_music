//! Playback of one track at a time, each in a task on its own thread, handing over to the
//! next without a gap.

mod control;
mod decoding;
mod playback;
mod successor;

use super::output::Output;
use crate::library::track::ReplayGainMode;
use crate::queue::ItemId;
use crate::source::{Locator, Providers};
use crate::TrackTime;
use control::{Completion, PlaybackControl};
use decoding::{Decoding, PlaybackSource};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct Player {
    volume: f32,
    replay_gain: ReplayGainMode,
    /// The device chosen to play on, see [`Output::open`].
    device: Option<String>,
    control: Option<PlaybackControl>,
    progress_interval: Option<Duration>,
    /// The device stream, kept open between tracks. A running task holds the lock.
    output: Arc<Mutex<Option<Output>>>,
}

impl Player {
    pub fn new(volume: f32, replay_gain: ReplayGainMode, device: Option<String>) -> Self {
        Self {
            volume,
            replay_gain,
            device,
            control: None,
            progress_interval: None,
            output: Arc::new(Mutex::new(None)),
        }
    }
    pub fn pause(&mut self) {
        if let Some(control) = &self.control {
            control.set_paused(true);
        }
    }
    pub fn unpause(&mut self) {
        if let Some(control) = &self.control {
            control.set_paused(false);
        }
    }
    /// Reopens the output, on the device it should be on now.
    pub fn reload_output(&self) {
        if let Some(control) = &self.control {
            control.reload_output();
        }
        // An idle stream would otherwise stay on the old device.
        if let Ok(mut output) = self.output.try_lock() {
            output.take();
        }
    }
    /// Plays on `device` while it is there, or (`None`) on the default device.
    pub fn set_output_device(&mut self, device: Option<String>) {
        self.device = device.clone();
        if let Some(control) = &self.control {
            control.set_output_device(device);
        }
        self.reload_output();
    }
    pub fn is_paused(&self) -> bool {
        self.control
            .as_ref()
            .is_some_and(PlaybackControl::is_paused)
    }
    /// Applies from the next track.
    pub fn set_replay_gain(&mut self, mode: ReplayGainMode) {
        self.replay_gain = mode;
    }
    pub fn volume(&self) -> f32 {
        self.volume
    }
    pub fn set_volume(&mut self, volume: f32) {
        if !volume.is_finite() {
            return;
        }
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(control) = &self.control {
            control.set_volume(self.volume);
        }
    }
    pub fn seek_to(&mut self, seconds: f64) {
        if let Some(control) = &self.control {
            control.seek(seconds);
        }
    }
    pub fn seek_revision(&self) -> u64 {
        self.control.as_ref().map_or(0, PlaybackControl::revision)
    }
    /// Where the running task is, exactly; position events are throttled.
    pub fn time(&self) -> Option<TrackTime> {
        self.control.as_ref().and_then(PlaybackControl::time)
    }
    pub fn is_playing(&self) -> bool {
        self.control
            .as_ref()
            .is_some_and(PlaybackControl::is_playing)
    }
    pub fn end_current(&mut self) {
        if let Some(control) = &self.control {
            control.stop();
        }
    }
    pub fn set_progress_interval(&mut self, interval: Option<Duration>) {
        self.progress_interval = interval;
        if let Some(control) = &self.control {
            control.set_progress_interval(interval);
        }
    }
    /// The track to play without a gap once the running one ends, see [`Next`].
    pub fn set_next(&self, next: Option<Next>) {
        if let Some(control) = &self.control {
            control.set_next(next);
        }
    }
    /// Prepares playing `locator`, the queue `item`.
    pub fn prepare_track(
        &mut self,
        providers: Arc<Providers>,
        locator: Locator,
        item: ItemId,
    ) -> PlaybackTask {
        self.end_current();
        let control = PlaybackControl::new(self.volume, self.device.clone());
        control.set_progress_interval(self.progress_interval);
        self.control = Some(control.clone());
        PlaybackTask {
            source: Some((PlaybackSource { providers, locator }, item)),
            control,
            output: self.output.clone(),
            replay_gain: self.replay_gain,
        }
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        self.end_current();
    }
}

/// A gapless successor: `locator`, the queue `item`, follows the track at `after`.
/// A task only takes it while it plays `after`, so a successor computed for an older
/// position is never used.
#[derive(Clone, Debug, PartialEq)]
pub struct Next {
    pub after: ItemId,
    pub item: ItemId,
    pub locator: Locator,
}

pub struct PlaybackTask {
    source: Option<(PlaybackSource, ItemId)>,
    control: PlaybackControl,
    output: Arc<Mutex<Option<Output>>>,
    replay_gain: ReplayGainMode,
}

impl PlaybackTask {
    pub fn run(mut self, mut emit: impl FnMut(PlaybackEvent)) -> Result<(), Failure> {
        let Some((source, item)) = self.source.take() else {
            return Ok(());
        };
        if !self.control.is_playing() {
            return Ok(());
        }
        // Open before taking the output: a slow source must not hold up the next task.
        let opened = Decoding::open(source, item, self.replay_gain);
        let mut output = self
            .output
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !self.control.is_playing() {
            return Ok(());
        }
        let result = opened
            .map_err(Failure::Track)
            .and_then(|current| self.play(current, &mut output, &mut emit));
        if result.is_err() {
            self.control.stop();
        }
        // Keep the stream for the next track, silent and paused.
        if let Some(stream) = output.as_mut() {
            stream.discard_queued();
            if stream.set_paused(true).is_err() {
                output.take();
            }
        }
        result
    }
}

impl PlaybackTask {
    fn play(
        &self,
        current: Decoding,
        output: &mut Option<Output>,
        emit: &mut impl FnMut(PlaybackEvent),
    ) -> Result<(), Failure> {
        if !self.control.begin() {
            return Ok(());
        }
        let completion = Completion(self.control.clone());
        let ended = playback::play(
            current,
            self.replay_gain,
            self.control.clone(),
            output,
            emit,
        )?;
        drop(completion);
        if ended {
            emit(PlaybackEvent::Ended);
        }
        Ok(())
    }
}

impl Drop for PlaybackTask {
    fn drop(&mut self) {
        if self.control.is_playing() {
            self.control.stop();
        }
    }
}

/// Events published by a running [`PlaybackTask`]. They are always delivered tagged with the
/// owning job, so the receiver can discard events from a replaced playback task.
pub enum PlaybackEvent {
    Started {
        length: f64,
        paused: bool,
    },
    /// The [`Next`] track became audible, right after the previous one.
    Advanced {
        item: ItemId,
        length: f64,
    },
    Position {
        time: TrackTime,
        revision: u64,
        discontinuity: bool,
    },
    Paused(bool),
    Ended,
    Failed(Failure),
}

/// Why a task stopped early.
#[derive(Debug)]
pub enum Failure {
    /// The track could not be opened, sought or decoded.
    Track(io::Error),
    /// The audio output failed for good.
    Output(io::Error),
}

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        Self::Track(error)
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Track(error) => write!(f, "track failed: {error}"),
            Self::Output(error) => write!(f, "audio output failed: {error}"),
        }
    }
}
