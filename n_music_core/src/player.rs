use crate::convert::Converter;
use crate::library::track::ReplayGainMode;
use crate::music_track::{replay_gain, MusicTrack};
use crate::output::{self, Output, OutputFormat};
use crate::queue::ItemId;
use crate::source::{Locator, Providers};
use crate::{TrackTime, CODEC_REGISTRY};
use std::io;
use std::sync::{Arc, Mutex};
use std::thread::{JoinHandle, Thread};
use std::time::{Duration, Instant};
use symphonia::core::audio::Channels;
use symphonia::core::codecs::audio::well_known::CODEC_ID_OPUS;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::units::{Time, TimeBase, Timestamp};

pub struct Player {
    volume: f32,
    replay_gain: ReplayGainMode,
    control: Option<PlaybackControl>,
    progress_interval: Option<Duration>,
    reported_end: bool,
    /// The device stream, kept open between tracks. A running task holds the lock.
    output: Arc<Mutex<Option<Output>>>,
}

impl Player {
    pub fn new(volume: f32, replay_gain: ReplayGainMode) -> Self {
        Self {
            volume,
            replay_gain,
            control: None,
            progress_interval: None,
            reported_end: false,
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
    /// Reopens the output on the current default device.
    pub fn reload_output(&self) {
        if let Some(control) = &self.control {
            control.reload_output();
        }
        // An idle stream would otherwise stay on the old device.
        if let Ok(mut output) = self.output.try_lock() {
            output.take();
        }
    }
    pub fn is_paused(&self) -> bool {
        self.control
            .as_ref()
            .is_some_and(PlaybackControl::is_paused)
    }
    pub fn get_volume(&self) -> f32 {
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
    pub fn seek_to(&mut self, seconds: u64, frac: f64) {
        if let Some(control) = &self.control {
            control.seek(seconds as f64 + frac);
        }
    }
    pub fn seek_revision(&self) -> u64 {
        self.control.as_ref().map_or(0, PlaybackControl::revision)
    }
    pub fn get_time(&self) -> Option<TrackTime> {
        self.control.as_ref().and_then(PlaybackControl::time)
    }
    pub fn has_ended(&mut self) -> bool {
        let ended = self
            .control
            .as_ref()
            .is_some_and(PlaybackControl::has_ended);
        let changed = ended && !self.reported_end;
        self.reported_end = ended;
        changed
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
        self.reported_end = false;
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
        let control = PlaybackControl::new(self.volume);
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
impl Default for Player {
    fn default() -> Self {
        Self::new(1.0, ReplayGainMode::Off)
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

#[derive(Clone)]
struct PlaybackSource {
    providers: Arc<Providers>,
    locator: Locator,
}

impl PlaybackSource {
    /// Opens the track from its first byte.
    fn open(&self) -> io::Result<Box<dyn FormatReader>> {
        MusicTrack::new(self.providers.as_ref(), &self.locator).get_format()
    }
}

pub struct PlaybackTask {
    source: Option<(PlaybackSource, ItemId)>,
    control: PlaybackControl,
    output: Arc<Mutex<Option<Output>>>,
    replay_gain: ReplayGainMode,
}

impl PlaybackTask {
    pub fn run(mut self, mut emit: impl FnMut(PlaybackEvent)) -> io::Result<()> {
        let mut output = self
            .output
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !self.control.is_playing() {
            return Ok(());
        }
        let (source, item) = self.source.take().unwrap();
        let result = run(
            source,
            item,
            self.replay_gain,
            self.control.clone(),
            &mut output,
            &mut emit,
        );
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
    Failed(String),
}

struct State {
    volume: f32,
    paused: bool,
    reload_output: bool,
    seek: Option<(Time, u64)>,
    revision: u64,
    version: u64,
    stopped: bool,
    running: bool,
    finished: bool,
    ended: bool,
    time: Option<(TrackTime, u64)>,
    progress_interval: Option<Duration>,
    next: Option<Next>,
    /// The task's thread, woken on every change.
    thread: Option<Thread>,
}

#[derive(Clone)]
struct PlaybackControl(Arc<Mutex<State>>);

struct Controls {
    volume: f32,
    paused: bool,
    reload_output: bool,
    seek: Option<(Time, u64)>,
    version: u64,
    stopped: bool,
    progress_interval: Option<Duration>,
    next: Option<Next>,
}

impl PlaybackControl {
    fn new(volume: f32) -> Self {
        Self(Arc::new(Mutex::new(State {
            volume,
            paused: false,
            reload_output: false,
            seek: None,
            revision: 0,
            version: 0,
            stopped: false,
            running: false,
            finished: false,
            ended: false,
            time: None,
            progress_interval: Some(Duration::from_millis(50)),
            next: None,
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

    fn set_paused(&self, paused: bool) {
        self.update(|state| state.paused = paused);
    }

    fn reload_output(&self) {
        self.update(|state| state.reload_output = true);
    }

    fn set_volume(&self, volume: f32) {
        self.update(|state| state.volume = volume.clamp(0.0, 1.0));
    }

    fn set_progress_interval(&self, interval: Option<Duration>) {
        self.update(|state| state.progress_interval = interval);
    }

    fn set_next(&self, next: Option<Next>) {
        self.update(|state| state.next = next);
    }

    fn seek(&self, seconds: f64) -> u64 {
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

    fn revision(&self) -> u64 {
        self.0.lock().unwrap().revision
    }

    fn time(&self) -> Option<TrackTime> {
        let state = self.0.lock().unwrap();
        state
            .time
            .filter(|(_, revision)| *revision == state.revision)
            .map(|(time, _)| time)
    }

    fn is_paused(&self) -> bool {
        self.0.lock().unwrap().paused
    }

    fn is_playing(&self) -> bool {
        let state = self.0.lock().unwrap();
        !state.stopped && !state.finished
    }

    fn has_ended(&self) -> bool {
        self.0.lock().unwrap().ended
    }

    fn stop(&self) {
        self.update(|state| {
            state.stopped = true;
            if !state.running {
                state.finished = true;
            }
        });
    }

    /// Registers the calling thread as the task's; false if it was stopped already.
    fn begin(&self) -> bool {
        let mut state = self.0.lock().unwrap();
        if state.stopped {
            return false;
        }
        state.running = true;
        state.thread = Some(std::thread::current());
        true
    }

    fn finish(&self, ended: bool) {
        let mut state = self.0.lock().unwrap();
        state.finished = true;
        state.running = false;
        state.thread = None;
        state.ended = ended && !state.stopped;
    }

    fn controls(&self) -> Controls {
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
        }
    }

    fn publish(&self, time: TrackTime, revision: u64) {
        self.0.lock().unwrap().time = Some((time, revision));
    }
}

struct Completion(PlaybackControl, bool);

impl Drop for Completion {
    fn drop(&mut self) {
        self.0.finish(self.1);
    }
}

/// A track being decoded and converted to the output format.
struct Decoding {
    source: PlaybackSource,
    item: ItemId,
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    time_base: TimeBase,
    length: f64,
    /// ReplayGain, as a linear factor.
    gain: f32,
    converter: Option<Converter>,
    /// Decoded frames before this are dropped: seeking lands on a packet boundary.
    seek_target: Option<Timestamp>,
    /// Audio was decoded since the last seek, so a new output has to rewind.
    decoded: bool,
    interleaved: Vec<f32>,
    /// The decoder leaves encoder delay and padding in (our Opus decoder), so they are
    /// trimmed here, and only where they can be real: Symphonia's Ogg reader flags whole
    /// packets mid-stream as padding when a file's granule positions are off.
    trims: bool,
    /// Encoder delay frames still to drop at the start of the stream.
    start_trim: usize,
    /// The last decoded packet, held back until it is known whether it is the final one,
    /// whose padding is then dropped.
    held: Vec<f32>,
    /// Rate, layout and padding frames of `held`.
    held_spec: Option<(u32, Channels, usize)>,
}

enum Decoded {
    Audio,
    Skipped,
    End,
}

impl Decoding {
    fn open(
        source: PlaybackSource,
        item: ItemId,
        replay_gain_mode: ReplayGainMode,
    ) -> io::Result<Self> {
        let mut format = source.open()?;
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| io::Error::other("No audio track"))?;
        let track_id = track.id;
        let time_base = track
            .time_base
            .ok_or_else(|| io::Error::other("No audio time base"))?;
        let length = track
            .duration
            .and_then(|duration| time_base.calc_duration(duration))
            .map(|time| time.as_secs_f64())
            .unwrap_or(0.0);
        let params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .ok_or_else(|| io::Error::other("No audio codec parameters"))?;
        let decoder = CODEC_REGISTRY
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .map_err(io::Error::other)?;
        let trims = params.codec == CODEC_ID_OPUS;
        let start_trim = if trims {
            track.delay.unwrap_or(0) as usize
        } else {
            0
        };
        let gain = replay_gain(format.as_mut(), track_id).factor(replay_gain_mode);
        Ok(Self {
            source,
            item,
            format,
            decoder,
            track_id,
            time_base,
            length,
            gain,
            converter: None,
            seek_target: None,
            decoded: false,
            interleaved: vec![],
            trims,
            start_trim,
            held: vec![],
            held_spec: None,
        })
    }

    /// Opens a track on another thread, so a slow source does not starve the output.
    fn prefetch(
        source: PlaybackSource,
        item: ItemId,
        replay_gain_mode: ReplayGainMode,
    ) -> JoinHandle<io::Result<Self>> {
        std::thread::spawn(move || Self::open(source, item, replay_gain_mode))
    }

    fn reopen(&mut self) -> io::Result<()> {
        self.format = self.source.open()?;
        self.decoder.reset();
        Ok(())
    }

    fn reset(&mut self) {
        self.decoder.reset();
        if let Some(converter) = &mut self.converter {
            converter.reset();
        }
        self.decoded = false;
        self.start_trim = 0;
        self.held.clear();
        self.held_spec = None;
    }

    /// Seeks to `target`; returns the position it landed on and whether the reader had to be
    /// rebuilt to get there. `rewind` reopens the source first: some demuxers cannot seek
    /// backward without an index, or after EOF.
    fn seek(
        &mut self,
        target: Time,
        rewind: bool,
        automatic: bool,
    ) -> io::Result<Option<(f64, bool)>> {
        if rewind {
            self.reopen()?;
        }
        let seeked = self.format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: target,
                track_id: Some(self.track_id),
            },
        );
        if (automatic && seeked.is_err())
            || (rewind
                && seeked
                    .as_ref()
                    .is_ok_and(|seeked| seeked.actual_ts > seeked.required_ts))
        {
            // An unindexed reader may only seek forward past the target packet.
            // Decode and discard from the start in that case, rather than skipping audio.
            self.reopen()?;
            self.reset();
            self.seek_target = Some(
                self.time_base
                    .calc_timestamp(target)
                    .ok_or_else(|| io::Error::other("Seek time out of range"))?,
            );
            return Ok(Some((target.as_secs_f64(), true)));
        }
        match seeked {
            Ok(seeked) => {
                self.reset();
                self.seek_target = Some(seeked.required_ts);
                let position = self
                    .time_base
                    .calc_time(seeked.required_ts)
                    .ok_or_else(|| io::Error::other("Seek time out of range"))?;
                Ok(Some((position.as_secs_f64(), false)))
            }
            Err(err) => {
                log::warn!("Could not seek to {target:?}: {err}");
                Ok(None)
            }
        }
    }

    /// Decodes one packet into `out`, converted to `output`.
    fn decode(&mut self, output: OutputFormat, out: &mut Vec<f32>) -> io::Result<Decoded> {
        let packet = match self.format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => return Ok(Decoded::End),
            Err(err) => return Err(io::Error::other(err)),
        };
        if packet.track_id != self.track_id {
            return Ok(Decoded::Skipped);
        }
        while !self.format.metadata().is_latest() {
            self.format.metadata().pop();
        }
        let decoded = match self.decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(symphonia::core::errors::Error::DecodeError(_)) => return Ok(Decoded::Skipped),
            Err(err) => return Err(io::Error::other(err)),
        };
        let packet_start = packet.pts.saturating_add(packet.trim_start);
        let skip_frames = match self.seek_target {
            Some(target) if packet_start < target => {
                let skip = self
                    .time_base
                    .calc_duration(target.abs_delta(packet_start))
                    .ok_or_else(|| io::Error::other("Seek duration out of range"))?;
                (skip.as_secs_f64() * decoded.spec().rate() as f64).round() as usize
            }
            _ => 0,
        };
        if skip_frames >= decoded.frames() {
            return Ok(Decoded::Skipped);
        }
        self.seek_target = None;
        let rate = decoded.spec().rate();
        let channels = decoded.spec().channels().clone();
        let frames = decoded.frames();
        decoded.copy_to_vec_interleaved(&mut self.interleaved);
        let mut start = skip_frames;
        let mut trim_end = 0;
        if self.trims {
            let trim = self.start_trim.min(frames);
            self.start_trim -= trim;
            start = start.max(trim);
            trim_end = self
                .time_base
                .calc_duration(packet.trim_end)
                .map_or(0, |time| {
                    (time.as_secs_f64() * rate as f64).round() as usize
                });
        }
        if start >= frames {
            return Ok(Decoded::Skipped);
        }
        self.interleaved.drain(..start * channels.count());
        self.release(output, out, false)?;
        std::mem::swap(&mut self.held, &mut self.interleaved);
        self.held_spec = Some((rate, channels, trim_end));
        self.decoded = true;
        Ok(Decoded::Audio)
    }

    /// Converts the held-back packet into `out`; `last` drops its padding.
    fn release(&mut self, output: OutputFormat, out: &mut Vec<f32>, last: bool) -> io::Result<()> {
        let Some((rate, channels, trim_end)) = self.held_spec.take() else {
            return Ok(());
        };
        let mut len = self.held.len();
        if last {
            len = len.saturating_sub(trim_end * channels.count());
        }
        if !self
            .converter
            .as_ref()
            .is_some_and(|converter| converter.fits(rate, &channels, output))
        {
            self.converter =
                Some(Converter::new(rate, &channels, output, self.gain).map_err(io::Error::other)?);
        }
        if let Some(converter) = &mut self.converter {
            converter.push(&self.held[..len], out);
        }
        self.held.clear();
        Ok(())
    }

    /// Converts everything still held at the end of the track.
    fn finish(&mut self, output: OutputFormat, out: &mut Vec<f32>) -> io::Result<()> {
        self.release(output, out, true)?;
        if let Some(converter) = &mut self.converter {
            converter.finish(out);
        }
        Ok(())
    }
}

/// A finished track still being heard while its successor is already queued.
struct Handover {
    previous: Decoding,
    /// Output frame where the successor starts.
    boundary: u64,
}

/// Maps output frames to the audible track's position.
#[derive(Clone, Copy)]
struct Timeline {
    /// Output frame playing `position`.
    start: u64,
    position: f64,
}

struct Prefetch {
    next: Next,
    track: JoinHandle<io::Result<Decoding>>,
}

/// The volume slider's curve.
fn output_volume(volume: f32) -> f32 {
    let volume = volume.clamp(0.0, 1.0);
    1.0 - (1.0 - volume * volume).sqrt()
}

fn run(
    source: PlaybackSource,
    item: ItemId,
    replay_gain_mode: ReplayGainMode,
    control: PlaybackControl,
    output: &mut Option<Output>,
    emit: &mut impl FnMut(PlaybackEvent),
) -> io::Result<()> {
    let mut current = Decoding::open(source, item, replay_gain_mode)?;
    if !control.begin() {
        return Ok(());
    }
    let mut completion = Completion(control.clone(), false);
    if let Some(stream) = output.as_ref() {
        if stream.check_health().is_ok() {
            stream.attach();
        } else {
            output.take();
        }
    }
    let mut handover: Option<Handover> = None;
    let mut prefetch: Option<Prefetch> = None;
    // A stream kept from the previous track has already played frames.
    let mut timeline = Timeline {
        start: output.as_ref().map_or(0, Output::produced_frames),
        position: 0.0,
    };
    // Converted samples not queued yet.
    let mut pending: Vec<f32> = vec![];
    let mut pending_offset = 0;
    let mut draining = false;
    let mut output_error = None;
    let mut recovering = false;
    let mut recovery_anchor = None;
    let mut retry_after = Instant::now();
    let mut paused = control.is_paused();
    let mut version = u64::MAX;
    let mut volume = f32::NAN;
    let mut revision = 0;
    let mut time = TrackTime {
        position: 0.0,
        length: current.length,
    };
    let mut last_report = Instant::now();
    let mut force_report = true;
    let mut progress_interval = Some(Duration::from_millis(50));
    emit(PlaybackEvent::Started {
        length: current.length,
        paused,
    });

    loop {
        let controls = control.controls();
        let changed = controls.version != version;
        version = controls.version;
        if controls.stopped {
            return Ok(());
        }
        if output_error.is_none() {
            output_error = output
                .as_ref()
                .and_then(|output| output.check_health().err());
        }
        if controls.progress_interval != progress_interval {
            progress_interval = controls.progress_interval;
            force_report = progress_interval.is_some();
        }
        if changed && controls.volume != volume {
            volume = controls.volume;
            if let Some(output) = output.as_ref() {
                output.set_volume(output_volume(volume));
            }
        }
        let pause_changed = controls.paused != paused;
        paused = controls.paused;
        if output_error.is_none() && !controls.reload_output {
            if let Some(output) = output.as_mut() {
                output_error = output.set_paused(paused).err();
            }
        }
        if pause_changed {
            force_report = true;
        }
        let mut reload_output = controls.reload_output;
        if let Some(error) = output_error.take() {
            if !error.is_recoverable() {
                return Err(io::Error::other(error));
            }
            if !recovering {
                log::warn!("Audio output unavailable, retrying the system default: {error}");
            }
            recovering = true;
            reload_output = true;
            retry_after = Instant::now() + Duration::from_millis(250);
        }
        let rewind = reload_output
            && (output.is_some() || current.decoded || handover.is_some() || !pending.is_empty());
        if reload_output {
            // Close before reopening: some backends require exclusive access. Rewind below
            // rather than skipping what was queued.
            output.take();
            force_report = true;
        }
        let recovery_seek = rewind.then(|| {
            (
                // Keep the original time until audio advances: seconds/timestamp round-trips
                // can lose a tick on each failed replacement stream at rates such as 44.1 kHz.
                *recovery_anchor.get_or_insert_with(|| {
                    Time::try_from_secs_f64(time.position).unwrap_or_default()
                }),
                controls
                    .seek
                    .as_ref()
                    .map_or(revision, |(_, revision)| *revision),
            )
        });
        // A user seek issued during recovery takes precedence over the recovery position.
        for (request, automatic) in [(controls.seek, false), (recovery_seek, true)] {
            let Some((target, next_revision)) = request else {
                continue;
            };
            revision = next_revision;
            force_report = true;
            // Seeking applies to what is heard: forget a successor that is only queued.
            if let Some(Handover { previous, .. }) = handover.take() {
                current = previous;
            }
            let start = output.as_ref().map_or(0, Output::produced_frames);
            if current.length > 0.0 && target.as_secs_f64() >= current.length {
                pending.clear();
                pending_offset = 0;
                if let Some(output) = output.as_mut() {
                    output.discard_queued();
                    output.set_draining();
                }
                time.position = current.length;
                timeline = Timeline {
                    start,
                    position: current.length,
                };
                draining = true;
                break;
            }
            let Some((position, rebuilt)) = current.seek(target, rewind, automatic)? else {
                continue;
            };
            if !automatic || rebuilt {
                recovery_anchor = Some(target);
            }
            pending.clear();
            pending_offset = 0;
            if let Some(output) = output.as_mut() {
                output.discard_queued();
            }
            time.position = position;
            timeline = Timeline { start, position };
            draining = false;
            break;
        }
        time.length = current.length;

        // Open the successor in the background as soon as it is known.
        let wanted = controls
            .next
            .clone()
            .filter(|next| handover.is_none() && next.after == current.item);
        if prefetch.as_ref().map(|prefetch| &prefetch.next) != wanted.as_ref() {
            prefetch = wanted.map(|next| Prefetch {
                track: Decoding::prefetch(
                    PlaybackSource {
                        providers: current.source.providers.clone(),
                        locator: next.locator.clone(),
                    },
                    next.item,
                    replay_gain_mode,
                ),
                next,
            });
        }

        if paused {
            if force_report {
                control.publish(time, revision);
                emit(PlaybackEvent::Position {
                    time,
                    revision,
                    discontinuity: true,
                });
                force_report = false;
            }
            if pause_changed {
                emit(PlaybackEvent::Paused(true));
            }
            // Stream callbacks only set atomics; check on the device now and then.
            std::thread::park_timeout(output::DEVICE_CHECK_INTERVAL);
            continue;
        }
        if pause_changed {
            emit(PlaybackEvent::Paused(false));
        }
        if output.is_none() {
            let now = Instant::now();
            if now < retry_after {
                if force_report {
                    control.publish(time, revision);
                    emit(PlaybackEvent::Position {
                        time,
                        revision,
                        discontinuity: true,
                    });
                    force_report = false;
                }
                std::thread::park_timeout(retry_after - now);
                continue;
            }
            match Output::open(output_volume(controls.volume)) {
                Ok(opened) => {
                    opened.attach();
                    volume = controls.volume;
                    *output = Some(opened);
                    force_report = true;
                }
                Err(error) => output_error = Some(error),
            }
            // Opening can take time; re-read the controls before playing.
            continue;
        }
        let Some(stream) = output.as_mut() else {
            continue;
        };
        let format = stream.format();

        // Where the listener is.
        let played = stream.played_frames();
        if let Some(boundary) = handover.as_ref().map(|handover| handover.boundary) {
            if played >= boundary {
                handover = None;
                timeline = Timeline {
                    start: boundary,
                    position: 0.0,
                };
                time.length = current.length;
                recovery_anchor = None;
                force_report = true;
                emit(PlaybackEvent::Advanced {
                    item: current.item,
                    length: current.length,
                });
            }
        }
        let audible_length = handover
            .as_ref()
            .map_or(current.length, |handover| handover.previous.length);
        time.position =
            timeline.position + played.saturating_sub(timeline.start) as f64 / format.rate as f64;
        if audible_length > 0.0 {
            time.position = time.position.min(audible_length);
        }
        if recovery_anchor.is_some_and(|anchor: Time| time.position > anchor.as_secs_f64()) {
            recovery_anchor = None;
        }
        control.publish(time, revision);
        if force_report
            || progress_interval.is_some_and(|interval| last_report.elapsed() >= interval)
        {
            emit(PlaybackEvent::Position {
                time,
                revision,
                discontinuity: force_report,
            });
            last_report = Instant::now();
            force_report = false;
        }
        // How long the thread may sleep before it has something to report.
        let mut timeout = output::DEVICE_CHECK_INTERVAL;
        if let Some(interval) = progress_interval {
            timeout = timeout.min(interval.saturating_sub(last_report.elapsed()));
        }
        if let Some(handover) = &handover {
            let frames = handover.boundary.saturating_sub(played);
            timeout = timeout.min(Duration::from_secs_f64(frames as f64 / format.rate as f64));
        }
        let timeout = timeout.max(Duration::from_millis(1));

        if pending_offset < pending.len() {
            match stream.write(&pending[pending_offset..]) {
                Ok(written) => pending_offset += written,
                Err(error) => {
                    output_error = Some(error);
                    continue;
                }
            }
            if pending_offset < pending.len() {
                stream.wait_for_room(timeout);
                continue;
            }
            pending.clear();
            pending_offset = 0;
            recovering = false;
        }

        if draining {
            // Join the successor if it arrived late.
            if let Some(next) = controls.next.filter(|next| next.after == current.item) {
                let track = match prefetch.take() {
                    Some(prefetch) if prefetch.next == next => prefetch.track.join(),
                    _ => Ok(Decoding::open(
                        PlaybackSource {
                            providers: current.source.providers.clone(),
                            locator: next.locator.clone(),
                        },
                        next.item,
                        replay_gain_mode,
                    )),
                };
                match track {
                    Ok(Ok(next)) => {
                        let previous = std::mem::replace(&mut current, next);
                        handover = Some(Handover {
                            previous,
                            boundary: stream.produced_frames(),
                        });
                        draining = false;
                        continue;
                    }
                    Ok(Err(error)) => {
                        log::warn!("Could not open the next track {}: {error}", next.locator)
                    }
                    Err(_) => log::error!("Opening the next track panicked"),
                }
            }
            let pending_frames = stream.pending_frames();
            if pending_frames == 0 {
                break;
            }
            stream.set_draining();
            let left = Duration::from_secs_f64(pending_frames as f64 / format.rate as f64);
            stream.wait_drained(timeout.min(left).max(Duration::from_millis(1)));
            continue;
        }

        match current.decode(format, &mut pending)? {
            Decoded::Audio | Decoded::Skipped => {}
            Decoded::End => {
                current.finish(format, &mut pending)?;
                // Queue what is left, then hand over or drain.
                draining = true;
                if pending.is_empty() {
                    continue;
                }
                // The successor starts after these samples; join it once they are queued.
            }
        }
    }
    time.position = time.length;
    control.publish(time, revision);
    emit(PlaybackEvent::Position {
        time,
        revision,
        discontinuity: true,
    });
    completion.1 = true;
    drop(completion);
    emit(PlaybackEvent::Ended);
    Ok(())
}
