use crate::{output, TrackTime, CODEC_REGISTRY};
use std::io::{self, Seek as _, SeekFrom};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use symphonia::core::audio::AudioSpec;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::units::Time;

use crate::music_track::MusicTrack;

pub struct Player {
    volume: f32,
    playback_speed: f32,
    control: Option<PlaybackControl>,
    progress_interval: Option<Duration>,
    reported_end: bool,
    output_access: Arc<Mutex<()>>,
}

impl Player {
    pub fn new(volume: f32, playback_speed: f32) -> Self {
        Self {
            volume,
            playback_speed,
            control: None,
            progress_interval: None,
            reported_end: false,
            output_access: Arc::new(Mutex::new(())),
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
    pub fn reload_output(&self) {
        if let Some(control) = &self.control {
            control.reload_output();
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
    pub fn set_playback_speed(&mut self, speed: f32) -> io::Result<()> {
        if !speed.is_finite() || speed <= 0.0 {
            return Err(io::Error::other("Invalid playback speed"));
        }
        self.playback_speed = speed;
        if let Some(control) = &self.control {
            control.set_speed(speed);
        }
        Ok(())
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
    fn prepare(&mut self, source: PlaybackSource) -> PlaybackTask {
        self.end_current();
        let control = PlaybackControl::new(self.volume, self.playback_speed);
        control.set_progress_interval(self.progress_interval);
        self.control = Some(control.clone());
        PlaybackTask {
            source: Some(source),
            control,
            output_access: self.output_access.clone(),
        }
    }
    pub fn prepare_path(&mut self, path: std::path::PathBuf) -> PlaybackTask {
        self.prepare(PlaybackSource::Path(path))
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        self.end_current();
    }
}
impl Default for Player {
    fn default() -> Self {
        Self::new(1.0, 1.0)
    }
}

enum PlaybackSource {
    Path(std::path::PathBuf),
}
pub struct PlaybackTask {
    source: Option<PlaybackSource>,
    control: PlaybackControl,
    output_access: Arc<Mutex<()>>,
}
impl PlaybackTask {
    pub fn run(mut self, mut emit: impl FnMut(PlaybackEvent)) -> io::Result<()> {
        let _output = self
            .output_access
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !self.control.is_playing() {
            return Ok(());
        }
        let result = (|| {
            let PlaybackSource::Path(path) = self.source.take().unwrap();
            let format = MusicTrack::new(path.to_string_lossy().into_owned())?.get_format()?;
            run(format, self.control.clone(), &mut emit)
        })();
        if result.is_err() {
            self.control.stop();
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
    speed: f32,
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
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

#[derive(Clone)]
struct PlaybackControl(Arc<Shared>);

struct Controls {
    volume: f32,
    speed: f32,
    paused: bool,
    reload_output: bool,
    seek: Option<(Time, u64)>,
    version: u64,
    stopped: bool,
    progress_interval: Option<Duration>,
}

impl PlaybackControl {
    pub fn new(volume: f32, speed: f32) -> Self {
        Self(Arc::new(Shared {
            state: Mutex::new(State {
                volume,
                speed,
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
            }),
            changed: Condvar::new(),
        }))
    }

    fn update(&self, change: impl FnOnce(&mut State)) {
        let mut state = self.0.state.lock().unwrap();
        change(&mut state);
        state.version = state.version.wrapping_add(1);
        self.0.changed.notify_all();
    }

    pub fn set_paused(&self, paused: bool) {
        self.update(|state| state.paused = paused);
    }

    fn reload_output(&self) {
        self.update(|state| state.reload_output = true);
    }

    pub fn set_volume(&self, volume: f32) {
        self.update(|state| state.volume = volume.clamp(0.0, 1.0));
    }

    pub fn set_speed(&self, speed: f32) {
        self.update(|state| state.speed = speed);
    }

    pub fn set_progress_interval(&self, interval: Option<Duration>) {
        self.update(|state| state.progress_interval = interval);
    }

    pub fn seek(&self, seconds: f64) -> u64 {
        let seconds = seconds.max(0.0);
        let mut state = self.0.state.lock().unwrap();
        state.revision = state.revision.wrapping_add(1);
        let revision = state.revision;
        state.seek = Some((
            Time::try_from_secs_f64(seconds).unwrap_or_default(),
            revision,
        ));
        state.version = state.version.wrapping_add(1);
        self.0.changed.notify_all();
        revision
    }

    pub fn revision(&self) -> u64 {
        self.0.state.lock().unwrap().revision
    }

    pub fn time(&self) -> Option<TrackTime> {
        let state = self.0.state.lock().unwrap();
        state
            .time
            .filter(|(_, revision)| *revision == state.revision)
            .map(|(time, _)| time)
    }

    pub fn is_paused(&self) -> bool {
        self.0.state.lock().unwrap().paused
    }

    pub fn is_playing(&self) -> bool {
        let state = self.0.state.lock().unwrap();
        !state.stopped && !state.finished
    }

    pub fn has_ended(&self) -> bool {
        self.0.state.lock().unwrap().ended
    }

    pub fn stop(&self) {
        self.update(|state| {
            state.stopped = true;
            if !state.running {
                state.finished = true;
            }
        });
    }

    fn begin(&self) -> bool {
        let mut state = self.0.state.lock().unwrap();
        if state.stopped {
            return false;
        }
        state.running = true;
        true
    }

    fn finish(&self, ended: bool) {
        let mut state = self.0.state.lock().unwrap();
        state.finished = true;
        state.running = false;
        state.ended = ended && !state.stopped;
        self.0.changed.notify_all();
    }

    fn controls(&self, version: u64, wait: bool) -> Controls {
        let mut state = self.0.state.lock().unwrap();
        while wait && state.version == version && !state.stopped {
            // Stream callbacks only set atomics; inspect them even while playback is paused.
            let (next, timeout) = self
                .0
                .changed
                .wait_timeout(state, output::DEVICE_CHECK_INTERVAL)
                .unwrap();
            state = next;
            if timeout.timed_out() {
                break;
            }
        }
        Controls {
            volume: state.volume,
            speed: state.speed,
            paused: state.paused,
            reload_output: std::mem::take(&mut state.reload_output),
            seek: state.seek.take(),
            version: state.version,
            stopped: state.stopped,
            progress_interval: state.progress_interval,
        }
    }

    fn wait(&self, version: u64, duration: Duration) {
        let state = self.0.state.lock().unwrap();
        if state.version == version && !state.stopped {
            drop(self.0.changed.wait_timeout(state, duration).unwrap());
        }
    }

    fn publish(&self, time: TrackTime, revision: u64) {
        self.0.state.lock().unwrap().time = Some((time, revision));
    }
}

struct Completion(PlaybackControl, bool);

impl Drop for Completion {
    fn drop(&mut self) {
        self.0.finish(self.1);
    }
}

fn rewind_format(format: Box<dyn FormatReader>) -> io::Result<Box<dyn FormatReader>> {
    let mut source = format.into_inner();
    source.seek(SeekFrom::Start(0))?;
    crate::PROBE
        .probe(
            &symphonia::core::formats::probe::Hint::new(),
            source,
            Default::default(),
            Default::default(),
        )
        .map_err(io::Error::other)
}

fn run(
    mut format: Box<dyn FormatReader>,
    control: PlaybackControl,
    emit: &mut impl FnMut(PlaybackEvent),
) -> io::Result<()> {
    if !control.begin() {
        return Ok(());
    }
    let mut completion = Completion(control.clone(), false);
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
    let mut decoder = CODEC_REGISTRY
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(io::Error::other)?;
    let mut output: Option<Box<dyn output::AudioOutput>> = None;
    let mut output_spec: Option<symphonia::core::audio::AudioSpec> = None;
    let mut output_capacity = 0;
    let mut output_source_rate = 0;
    let mut packet_needs_rewind = false;
    let mut output_error = None;
    let mut recovering = false;
    let mut recovery_anchor = None;
    let mut retry_after = Instant::now();
    let mut paused = control.is_paused();
    let mut version = u64::MAX;
    let mut revision = 0;
    let mut seek_target = None;
    let mut position_floor = 0.0;
    let mut time = TrackTime {
        position: 0.0,
        length,
    };
    let mut last_report = Instant::now();
    let mut force_report = true;
    let mut last_volume = f32::NAN;
    let mut output_volume = 0.0;
    let mut draining = false;
    let mut progress_interval = Some(Duration::from_millis(50));
    emit(PlaybackEvent::Started { length, paused });

    loop {
        let controls = control.controls(version, paused && !force_report);
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
        let pause_changed = controls.paused != paused;
        paused = controls.paused;
        if pause_changed {
            if output_error.is_none() && !controls.reload_output {
                if let Some(output) = &mut output {
                    output_error = output.set_paused(paused).err();
                }
            }
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
        let rewind = reload_output && (output.is_some() || packet_needs_rewind);
        if reload_output {
            // Close before reopening (some backends require exclusive access). Rewind below
            // rather than skipping the old queue or the remainder of a partially written packet.
            drop(output.take());
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
            if length > 0.0 && target.as_secs_f64() >= length {
                if let Some(output) = &mut output {
                    output.discard_queued();
                    output.set_draining();
                }
                time.position = length;
                seek_target = None;
                packet_needs_rewind = false;
                draining = true;
                break;
            }
            if rewind {
                // Some demuxers cannot seek backward without an index, or after EOF.
                // Rebuild once from the source, not on every failed device-open attempt.
                format = rewind_format(format)?;
                decoder.reset();
            }
            let seeked = format.seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: target,
                    track_id: Some(track_id),
                },
            );
            if (automatic && seeked.is_err())
                || (rewind
                    && seeked
                        .as_ref()
                        .is_ok_and(|seeked| seeked.actual_ts > seeked.required_ts))
            {
                // An unindexed reader may only seek forward past the target packet.
                // Decode/discard from the start in that case, rather than skipping audio.
                format = rewind_format(format)?;
                decoder.reset();
                seek_target = Some(
                    time_base
                        .calc_timestamp(target)
                        .ok_or_else(|| io::Error::other("Seek time out of range"))?,
                );
                position_floor = target.as_secs_f64();
                time.position = position_floor;
                recovery_anchor = Some(target);
                packet_needs_rewind = false;
                draining = false;
                break;
            }
            match seeked {
                Ok(seeked) => {
                    if !automatic {
                        recovery_anchor = Some(target);
                    }
                    decoder.reset();
                    if let Some(output) = &mut output {
                        output.discard_queued();
                    }
                    seek_target = Some(seeked.required_ts);
                    let position = time_base
                        .calc_time(seeked.required_ts)
                        .ok_or_else(|| io::Error::other("Seek time out of range"))?;
                    position_floor = position.as_secs_f64();
                    time.position = position_floor;
                    draining = false;
                    packet_needs_rewind = false;
                    break;
                }
                Err(err) => log::warn!("Could not seek to {target:?}: {err}"),
            }
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
            continue;
        }
        if pause_changed {
            emit(PlaybackEvent::Paused(false));
        }
        if output.is_none() && Instant::now() < retry_after {
            if force_report {
                control.publish(time, revision);
                emit(PlaybackEvent::Position {
                    time,
                    revision,
                    discontinuity: true,
                });
                force_report = false;
            }
            control.wait(
                version,
                retry_after.saturating_duration_since(Instant::now()),
            );
            continue;
        }
        if draining {
            if let (Some(output), Some(spec)) = (&output, &output_spec) {
                let pending = output.pending_frames();
                if pending > 0 {
                    control.wait(
                        version,
                        Duration::from_secs_f64(pending as f64 / spec.rate() as f64)
                            .min(Duration::from_millis(50)),
                    );
                    continue;
                }
            }
            break;
        }
        if output.is_none() {
            if let Some(spec) = &output_spec {
                let spec = AudioSpec::new(
                    (output_source_rate as f32 * controls.speed).round() as u32,
                    spec.channels().clone(),
                );
                output_spec = Some(spec.clone());
                match output::try_open(spec, output_capacity) {
                    Ok(opened) => {
                        output = Some(opened);
                        force_report = true;
                        // Opening can take time; re-read transport controls before decoding.
                        continue;
                    }
                    Err(error) => {
                        output_error = Some(error);
                        continue;
                    }
                }
            }
        }
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => {
                if let Some(output) = &mut output {
                    output.set_draining();
                }
                draining = true;
                continue;
            }
            Err(err) => return Err(io::Error::other(err)),
        };
        if packet.track_id != track_id {
            continue;
        }
        while !format.metadata().is_latest() {
            format.metadata().pop();
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(err) => return Err(io::Error::other(err)),
        };
        let packet_start = packet.pts.saturating_add(packet.trim_start);
        let skip_frames = match seek_target {
            Some(target) if packet_start < target => {
                let skip = time_base
                    .calc_duration(target.abs_delta(packet_start))
                    .ok_or_else(|| io::Error::other("Seek duration out of range"))?;
                (skip.as_secs_f64() * decoded.spec().rate() as f64).round() as usize
            }
            _ => 0,
        };
        if skip_frames >= decoded.frames() {
            continue;
        }
        seek_target = None;
        let frames = decoded.frames();
        let source_rate = decoded.spec().rate();
        let spec = symphonia::core::audio::AudioSpec::new(
            (source_rate as f32 * controls.speed).round() as u32,
            decoded.spec().channels().clone(),
        );
        let capacity = decoded.capacity();
        packet_needs_rewind = true;
        if output.is_none() || output_spec.as_ref() != Some(&spec) || output_capacity < capacity {
            drop(output.take());
            output_spec = Some(spec.clone());
            output_source_rate = source_rate;
            output_capacity = capacity;
            match output::try_open(spec.clone(), capacity) {
                Ok(opened) => {
                    output = Some(opened);
                    force_report = true;
                }
                Err(error) => {
                    output_error = Some(error);
                    continue;
                }
            }
        }
        if let Some(output) = &mut output {
            if last_volume != controls.volume {
                last_volume = controls.volume;
                let volume = controls.volume.clamp(0.0, 1.0);
                output_volume = 1.0 - (1.0 - volume * volume).sqrt();
            }
            if let Err(error) = output.write(decoded, output_volume, skip_frames) {
                output_error = Some(error);
                continue;
            }
            recovering = false;
            packet_needs_rewind = false;
            let start = time_base
                .calc_time(packet_start)
                .ok_or_else(|| io::Error::other("Packet time out of range"))?;
            time.position = (start.as_secs_f64() + frames as f64 / source_rate as f64
                - output.pending_frames() as f64 / source_rate as f64)
                .max(position_floor);
            if recovery_anchor.is_some_and(|anchor| time.position > anchor.as_secs_f64()) {
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
        }
    }
    time.position = length;
    control.publish(time, revision);
    emit(PlaybackEvent::Position {
        time,
        revision,
        discontinuity: true,
    });
    drop(output);
    completion.1 = true;
    drop(completion);
    emit(PlaybackEvent::Ended);
    Ok(())
}
