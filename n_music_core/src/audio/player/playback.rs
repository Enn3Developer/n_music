//! The playback loop of a task: applies the controls, keeps the output fed, follows what is
//! heard and hands over to the next track without a gap, or fades into it.

use super::super::output::{self, AudioOutputError, Output, OutputFormat};
use super::control::{Controls, PlaybackControl};
use super::decoding::{Decoded, Decoding, PlaybackSource};
use super::successor::Successor;
use super::{Failure, Next, PlaybackEvent};
use crate::library::track::ReplayGainMode;
use crate::TrackTime;
use std::io;
use std::time::{Duration, Instant};
use symphonia::core::units::Time;

/// The previous track while it is still heard: its end queued before the current track
/// (gapless), or fading out under its start (crossfade).
struct Handover {
    previous: Decoding,
    /// Output frame where the current track starts.
    boundary: u64,
    /// The boundary was heard, and the current track announced.
    advanced: bool,
    /// The previous track's end, while it is mixed under the current track's start.
    fade: Option<Fade>,
}

/// A crossfade being mixed: the previous track fades out as the current one fades in.
struct Fade {
    /// The previous track's converted samples not mixed in yet.
    samples: Vec<f32>,
    /// Frames mixed so far, of `frames`.
    done: usize,
    frames: usize,
    /// The previous track was decoded to its end: silence follows it.
    ended: bool,
}

impl Fade {
    /// The previous and the current track's gains at frame `at`: equal power, so the loudness
    /// holds through the fade.
    fn gains(&self, at: usize) -> (f32, f32) {
        let angle = at as f32 / self.frames as f32 * std::f32::consts::FRAC_PI_2;
        (angle.cos(), angle.sin())
    }
}

/// Maps output frames to the audible track's position.
#[derive(Clone, Copy)]
struct Timeline {
    /// Output frame playing `position`.
    start: u64,
    position: f64,
}

/// Converted samples, queued up to `offset`.
#[derive(Default)]
struct Pending {
    samples: Vec<f32>,
    offset: usize,
}

impl Pending {
    fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    fn clear(&mut self) {
        self.samples.clear();
        self.offset = 0;
    }
}

/// How the output is doing after a failure.
struct Recovery {
    error: Option<AudioOutputError>,
    /// The output failed and is being replaced.
    active: bool,
    /// Where to resume once a replaced output works, until audio advances past it.
    anchor: Option<Time>,
    retry_after: Instant,
}

/// When the position is reported.
struct Reporting {
    interval: Option<Duration>,
    last: Instant,
    /// Report at the next chance, as a discontinuity.
    force: bool,
}

/// The volume slider's curve.
fn output_volume(volume: f32) -> f32 {
    let volume = volume.clamp(0.0, 1.0);
    1.0 - (1.0 - volume * volume).sqrt()
}

/// Plays `current` until it ends (true) or the task is stopped (false).
pub(super) fn play<E: FnMut(PlaybackEvent)>(
    current: Decoding,
    replay_gain_mode: ReplayGainMode,
    control: PlaybackControl,
    output: &mut Option<Output>,
    emit: &mut E,
) -> Result<bool, Failure> {
    Playback::start(current, replay_gain_mode, control, output, emit).run()
}

/// Whether the loop goes on to the next step or starts over with fresh controls.
enum Step {
    Go,
    Again,
}

/// A running task's state between rounds of [`Playback::run`]: what is decoded, queued and
/// heard, and how the output is doing.
struct Playback<'a, E: FnMut(PlaybackEvent)> {
    control: PlaybackControl,
    output: &'a mut Option<Output>,
    emit: &'a mut E,
    replay_gain_mode: ReplayGainMode,
    current: Decoding,
    handover: Option<Handover>,
    successor: Option<Successor>,
    timeline: Timeline,
    pending: Pending,
    /// The current track is decoded to the end: what is queued plays out, or the successor
    /// takes over.
    draining: bool,
    recovery: Recovery,
    reporting: Reporting,
    paused: bool,
    /// The controls' version last applied.
    version: u64,
    volume: f32,
    /// The seek revision positions are reported for.
    revision: u64,
    time: TrackTime,
    /// How long the current track fades into the next.
    crossfade: Duration,
}

impl<'a, E: FnMut(PlaybackEvent)> Playback<'a, E> {
    /// Takes over the stream kept from the previous track, if it still works.
    fn start(
        current: Decoding,
        replay_gain_mode: ReplayGainMode,
        control: PlaybackControl,
        output: &'a mut Option<Output>,
        emit: &'a mut E,
    ) -> Self {
        if let Some(stream) = output.as_ref() {
            if stream.check_health().is_ok() {
                stream.attach();
            } else {
                output.take();
            }
        }
        let paused = control.is_paused();
        let length = current.length;
        // A stream kept from the previous track has already played frames.
        let timeline = Timeline {
            start: output.as_ref().map_or(0, Output::produced_frames),
            position: 0.0,
        };
        emit(PlaybackEvent::Started { length, paused });
        let now = Instant::now();
        Self {
            control,
            output,
            emit,
            replay_gain_mode,
            current,
            handover: None,
            successor: None,
            timeline,
            pending: Pending::default(),
            draining: false,
            recovery: Recovery {
                error: None,
                active: false,
                anchor: None,
                retry_after: now,
            },
            reporting: Reporting {
                interval: None,
                last: now,
                force: true,
            },
            paused,
            version: u64::MAX,
            volume: f32::NAN,
            revision: 0,
            time: TrackTime {
                position: 0.0,
                length,
            },
            crossfade: Duration::ZERO,
        }
    }

    /// Plays until the task is stopped (false) or the track ended (true).
    fn run(mut self) -> Result<bool, Failure> {
        loop {
            let controls = self.control.controls();
            if controls.stopped {
                return Ok(false);
            }
            let pause_changed = self.apply(&controls);
            let rewind = self.check_output(controls.reload_output)?;
            self.seek(controls.seek, rewind)?;
            self.time.length = self.current.length;
            self.update_successor(controls.next);

            if self.paused {
                if self.reporting.force {
                    self.report_now();
                }
                if pause_changed {
                    (self.emit)(PlaybackEvent::Paused(true));
                }
                // Stream callbacks only set atomics; check on the device now and then.
                std::thread::park_timeout(output::DEVICE_CHECK_INTERVAL);
                continue;
            }
            if pause_changed {
                (self.emit)(PlaybackEvent::Paused(false));
            }
            if self.output.is_none() {
                self.open_output(controls.volume);
                continue;
            }

            let timeout = self.follow_position();
            if let Step::Again = self.write_pending(timeout) {
                continue;
            }
            if self.draining {
                if self.drain(timeout) {
                    self.time.position = self.time.length;
                    self.report_now();
                    return Ok(true);
                }
                continue;
            }
            self.decode()?;
        }
    }

    /// Applies volume, pause and progress changes; returns whether pausing changed.
    fn apply(&mut self, controls: &Controls) -> bool {
        let changed = controls.version != self.version;
        self.version = controls.version;
        self.crossfade = controls.crossfade;
        if self.recovery.error.is_none() {
            self.recovery.error = self
                .output
                .as_ref()
                .and_then(|output| output.check_health().err());
        }
        if controls.progress_interval != self.reporting.interval {
            self.reporting.interval = controls.progress_interval;
            self.reporting.force = self.reporting.interval.is_some();
        }
        if changed && controls.volume != self.volume {
            self.volume = controls.volume;
            if let Some(output) = self.output.as_ref() {
                output.set_volume(output_volume(self.volume));
            }
        }
        let pause_changed = controls.paused != self.paused;
        self.paused = controls.paused;
        if self.recovery.error.is_none() && !controls.reload_output {
            if let Some(output) = self.output.as_mut() {
                self.recovery.error = output.set_paused(self.paused).err();
            }
        }
        if pause_changed {
            self.reporting.force = true;
        }
        pause_changed
    }

    /// Closes the output when it failed or a reload was asked for. Returns whether the track
    /// has to be rewound to what is heard, as queued audio was lost with it.
    fn check_output(&mut self, reload: bool) -> Result<bool, Failure> {
        let mut reload = reload;
        if let Some(error) = self.recovery.error.take() {
            if !error.is_recoverable() {
                return Err(Failure::Output(io::Error::other(error)));
            }
            if !self.recovery.active {
                log::warn!("Audio output unavailable, opening it again: {error}");
            }
            self.recovery.active = true;
            reload = true;
            self.recovery.retry_after = Instant::now() + Duration::from_millis(250);
        }
        let rewind = reload
            && (self.output.is_some()
                || self.current.decoded
                || self.handover.is_some()
                || !self.pending.is_empty());
        if reload {
            // Close before reopening: some backends require exclusive access. Rewind
            // rather than skipping what was queued.
            self.output.take();
            self.reporting.force = true;
        }
        Ok(rewind)
    }

    /// Applies a user seek, or the rewind after the output was replaced.
    fn seek(&mut self, user: Option<(Time, u64)>, rewind: bool) -> io::Result<()> {
        let recovery = rewind.then(|| {
            let position = self.time.position;
            (
                // Keep the original time until audio advances: seconds/timestamp round-trips
                // can lose a tick on each failed replacement stream at rates such as 44.1 kHz.
                *self
                    .recovery
                    .anchor
                    .get_or_insert_with(|| Time::try_from_secs_f64(position).unwrap_or_default()),
                user.as_ref()
                    .map_or(self.revision, |(_, revision)| *revision),
            )
        });
        // A user seek issued during recovery takes precedence over the recovery position.
        for (request, automatic) in [(user, false), (recovery, true)] {
            let Some((target, revision)) = request else {
                continue;
            };
            self.revision = revision;
            self.reporting.force = true;
            // Seeking applies to what is heard: forget a successor that is only queued, or the
            // end of the previous track fading out.
            if let Some(handover) = self.handover.take() {
                if !handover.advanced {
                    self.current = handover.previous;
                }
            }
            let start = self.output.as_ref().map_or(0, Output::produced_frames);
            if self.current.length > 0.0 && target.as_secs_f64() >= self.current.length {
                self.discard_pending();
                self.time.position = self.current.length;
                self.timeline = Timeline {
                    start,
                    position: self.current.length,
                };
                self.draining = true;
                break;
            }
            let Some((position, rebuilt)) = self.current.seek(target, rewind, automatic)? else {
                continue;
            };
            if !automatic || rebuilt {
                self.recovery.anchor = Some(target);
            }
            self.discard_pending();
            self.time.position = position;
            self.timeline = Timeline { start, position };
            self.draining = false;
            break;
        }
        Ok(())
    }

    /// Drops audio that is converted or queued but not heard yet.
    fn discard_pending(&mut self) {
        self.pending.clear();
        if let Some(output) = self.output.as_mut() {
            output.discard_queued();
        }
    }

    /// Starts opening the track after the current one as soon as it is known. A successor
    /// that failed stays failed while the queue still wants it.
    fn update_successor(&mut self, next: Option<Next>) {
        let wanted = next.filter(|next| next.after == self.current.item);
        if self.successor.as_ref().map(Successor::next) != wanted.as_ref() {
            self.successor = wanted.map(|next| {
                let source = PlaybackSource {
                    providers: self.current.source.providers.clone(),
                    locator: next.locator.clone(),
                };
                Successor::open(next, source, self.replay_gain_mode)
            });
        }
    }

    /// Opens the output device, once the retry delay after a failure has passed.
    fn open_output(&mut self, volume: f32) {
        let now = Instant::now();
        if now < self.recovery.retry_after {
            if self.reporting.force {
                self.report_now();
            }
            std::thread::park_timeout(self.recovery.retry_after - now);
            return;
        }
        match Output::open(
            output_volume(volume),
            self.control.output_device().as_deref(),
        ) {
            Ok(opened) => {
                opened.attach();
                self.volume = volume;
                *self.output = Some(opened);
                self.reporting.force = true;
            }
            Err(error) => self.recovery.error = Some(error),
        }
        // Opening can take time; the caller re-reads the controls before playing.
    }

    /// Publishes the position right away, as a discontinuity.
    fn report_now(&mut self) {
        self.control.publish(self.time, self.revision, false);
        (self.emit)(PlaybackEvent::Position {
            time: self.time,
            revision: self.revision,
            discontinuity: true,
        });
        self.reporting.force = false;
    }

    /// Works out what the listener hears, crossing into the successor when its first frame
    /// plays, and reports it. Returns how long the thread may wait before reporting again.
    fn follow_position(&mut self) -> Duration {
        let Some(stream) = self.output.as_ref() else {
            return Duration::from_millis(1);
        };
        let rate = stream.format().rate as f64;
        let played = stream.played_frames();
        let boundary = self
            .handover
            .as_ref()
            .filter(|handover| !handover.advanced)
            .map(|handover| handover.boundary);
        if let Some(boundary) = boundary.filter(|&boundary| played >= boundary) {
            if let Some(handover) = self.handover.as_mut() {
                handover.advanced = true;
            }
            self.timeline = Timeline {
                start: boundary,
                position: 0.0,
            };
            self.recovery.anchor = None;
            self.reporting.force = true;
            (self.emit)(PlaybackEvent::Advanced {
                item: self.current.item,
                length: self.current.length,
            });
        }
        // The previous track goes once the current one is heard and none of it is mixed in.
        if self
            .handover
            .as_ref()
            .is_some_and(|handover| handover.advanced && handover.fade.is_none())
        {
            self.handover = None;
        }
        let audible_length = match &self.handover {
            Some(handover) if !handover.advanced => handover.previous.length,
            _ => self.current.length,
        };
        self.time.length = audible_length;
        self.time.position =
            self.timeline.position + played.saturating_sub(self.timeline.start) as f64 / rate;
        if audible_length > 0.0 {
            self.time.position = self.time.position.min(audible_length);
        }
        if self
            .recovery
            .anchor
            .is_some_and(|anchor| self.time.position > anchor.as_secs_f64())
        {
            self.recovery.anchor = None;
        }
        self.control.publish(self.time, self.revision, true);
        if self.reporting.force
            || self
                .reporting
                .interval
                .is_some_and(|interval| self.reporting.last.elapsed() >= interval)
        {
            (self.emit)(PlaybackEvent::Position {
                time: self.time,
                revision: self.revision,
                discontinuity: self.reporting.force,
            });
            self.reporting.last = Instant::now();
            self.reporting.force = false;
        }
        let mut timeout = output::DEVICE_CHECK_INTERVAL;
        if let Some(interval) = self.reporting.interval {
            timeout = timeout.min(interval.saturating_sub(self.reporting.last.elapsed()));
        }
        if let Some(boundary) = boundary {
            let frames = boundary.saturating_sub(played);
            timeout = timeout.min(Duration::from_secs_f64(frames as f64 / rate));
        }
        timeout.max(Duration::from_millis(1))
    }

    /// Queues converted audio; [`Step::Again`] while some is left or the output failed.
    fn write_pending(&mut self, timeout: Duration) -> Step {
        let pending = &mut self.pending;
        if pending.offset >= pending.samples.len() {
            return Step::Go;
        }
        let Some(stream) = self.output.as_mut() else {
            return Step::Again;
        };
        match stream.write(&pending.samples[pending.offset..]) {
            Ok(written) => pending.offset += written,
            Err(error) => {
                self.recovery.error = Some(error);
                return Step::Again;
            }
        }
        if pending.offset < pending.samples.len() {
            stream.wait_for_room(timeout);
            return Step::Again;
        }
        pending.clear();
        self.recovery.active = false;
        Step::Go
    }

    /// Hands over to the successor once it is open, or waits for the queued audio to play
    /// out. Returns whether the track ended. A successor still opening when the queue ran dry
    /// is waited for, a gap rather than a skip.
    fn drain(&mut self, timeout: Duration) -> bool {
        if self.handover.is_none() {
            if let Some(track) = self.successor.as_mut().and_then(Successor::take) {
                self.successor = None;
                let previous = std::mem::replace(&mut self.current, track);
                self.handover = Some(Handover {
                    previous,
                    boundary: self.output.as_ref().map_or(0, Output::produced_frames),
                    advanced: false,
                    fade: None,
                });
                self.draining = false;
                return false;
            }
        }
        let opening = self.successor.as_mut().is_some_and(Successor::is_opening);
        let Some(stream) = self.output.as_mut() else {
            return false;
        };
        let pending_frames = stream.pending_frames();
        if pending_frames == 0 {
            if !opening {
                return true;
            }
            // The successor's thread wakes this one once it is open.
            std::thread::park_timeout(timeout);
            return false;
        }
        let left = Duration::from_secs_f64(pending_frames as f64 / stream.format().rate as f64);
        stream.wait_drained(timeout.min(left).max(Duration::from_millis(1)));
        false
    }

    /// Decodes the next packet, fading it in over the previous track's end while a crossfade
    /// is mixed; at the end, flushes what is held back and starts draining.
    fn decode(&mut self) -> io::Result<()> {
        let Some(format) = self.output.as_ref().map(Output::format) else {
            return Ok(());
        };
        if self.handover.is_none() {
            self.start_fade(format);
        }
        let start = self.pending.samples.len();
        if let Decoded::End = self.current.decode(format, &mut self.pending.samples)? {
            self.current.finish(format, &mut self.pending.samples)?;
            // Queue what is left, then hand over or drain; the successor starts after it.
            self.draining = true;
        }
        self.mix_fade(format, start);
        Ok(())
    }

    /// Starts fading into the successor once the current track is within the crossfade of its
    /// end, or within what is left of it when the successor opens later. Each fade takes at
    /// most half of either track.
    fn start_fade(&mut self, format: OutputFormat) {
        let length = self.current.length;
        if self.crossfade.is_zero() || length <= 0.0 {
            return;
        }
        let rate = f64::from(format.rate);
        // Everything decoded is queued by now.
        let queued = self.output.as_ref().map_or(0, Output::produced_frames);
        let decoded =
            self.timeline.position + queued.saturating_sub(self.timeline.start) as f64 / rate;
        let left = length - decoded;
        let fade = self.crossfade.as_secs_f64().min(length / 2.0);
        if left > fade {
            return;
        }
        let Some(next_length) = self.successor.as_mut().and_then(Successor::length) else {
            return;
        };
        let fade = if next_length > 0.0 {
            fade.min(next_length / 2.0)
        } else {
            fade
        };
        let frames = (fade.min(left) * rate) as usize;
        if frames == 0 {
            return;
        }
        let Some(track) = self.successor.as_mut().and_then(Successor::take) else {
            return;
        };
        self.successor = None;
        let previous = std::mem::replace(&mut self.current, track);
        self.handover = Some(Handover {
            previous,
            boundary: queued,
            advanced: false,
            fade: Some(Fade {
                samples: vec![],
                done: 0,
                frames,
                ended: false,
            }),
        });
    }

    /// Fades the current track's samples from `start` in, mixing the previous track's end in
    /// as it fades out.
    fn mix_fade(&mut self, format: OutputFormat, start: usize) {
        let Some(handover) = self.handover.as_mut() else {
            return;
        };
        let Some(fade) = handover.fade.as_mut() else {
            return;
        };
        let channels = format.channels;
        let incoming = &mut self.pending.samples[start..];
        let frames = (incoming.len() / channels).min(fade.frames - fade.done);
        while fade.samples.len() < frames * channels && !fade.ended {
            match handover.previous.decode(format, &mut fade.samples) {
                Ok(Decoded::End) => {
                    if let Err(error) = handover.previous.finish(format, &mut fade.samples) {
                        log::warn!("Could not finish decoding the fading track: {error}");
                    }
                    fade.ended = true;
                }
                Ok(_) => {}
                Err(error) => {
                    log::warn!("Could not decode the fading track: {error}");
                    fade.ended = true;
                }
            }
        }
        for (index, frame) in incoming[..frames * channels]
            .chunks_exact_mut(channels)
            .enumerate()
        {
            let (out_gain, in_gain) = fade.gains(fade.done + index);
            for (channel, sample) in frame.iter_mut().enumerate() {
                let previous = fade.samples.get(index * channels + channel).copied();
                *sample = *sample * in_gain + previous.unwrap_or(0.0) * out_gain;
            }
        }
        fade.samples
            .drain(..(frames * channels).min(fade.samples.len()));
        fade.done += frames;
        if fade.done >= fade.frames {
            handover.fade = None;
        }
    }
}
