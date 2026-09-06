use crate::{output, TrackTime, CODEC_REGISTRY};
use std::io;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo};
use symphonia::core::units::Time;

use crate::music_track::MusicTrack;
use std::path::Path;
use std::thread;

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
    pub fn play_from_path<P: AsRef<Path>>(&mut self, path: P) {
        self.prepare_path(path.as_ref().to_owned()).spawn();
    }
    pub fn play_from_track(&mut self, track: &MusicTrack) -> io::Result<()> {
        self.play(track.get_format()?);
        Ok(())
    }
    pub fn play(&mut self, format: Box<dyn FormatReader>) {
        self.prepare(PlaybackSource::Format(format)).spawn();
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
    Format(Box<dyn FormatReader>),
}
pub struct PlaybackTask {
    source: Option<PlaybackSource>,
    control: PlaybackControl,
    output_access: Arc<Mutex<()>>,
}
impl PlaybackTask {
    pub fn run(mut self, emit: impl FnMut(PlaybackEvent)) -> io::Result<()> {
        let _output = self
            .output_access
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !self.control.is_playing() {
            return Ok(());
        }
        let result = (|| {
            let format = match self.source.take().unwrap() {
                PlaybackSource::Path(path) => {
                    MusicTrack::new(path.to_string_lossy().into_owned())?.get_format()?
                }
                PlaybackSource::Format(format) => format,
            };
            run(format, self.control.clone(), emit)
        })();
        if result.is_err() {
            self.control.stop();
        }
        result
    }
    pub fn spawn(self) {
        thread::spawn(move || {
            if let Err(error) = self.run(|_| {}) {
                eprintln!("error playing track: {error}");
            }
        });
    }
}

impl Drop for PlaybackTask {
    fn drop(&mut self) {
        if self.control.is_playing() {
            self.control.stop();
        }
    }
}

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
        state.seek = Some((Time::from(seconds), revision));
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
            state = self.0.changed.wait(state).unwrap();
        }
        Controls {
            volume: state.volume,
            speed: state.speed,
            paused: state.paused,
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

fn run(
    mut format: Box<dyn FormatReader>,
    control: PlaybackControl,
    mut emit: impl FnMut(PlaybackEvent),
) -> io::Result<()> {
    if !control.begin() {
        return Ok(());
    }
    let mut completion = Completion(control.clone(), false);
    let track = format
        .default_track()
        .ok_or_else(|| io::Error::other("No audio track"))?;
    let track_id = track.id;
    let time_base = track
        .codec_params
        .time_base
        .ok_or_else(|| io::Error::other("No audio time base"))?;
    let duration = track.codec_params.n_frames.unwrap_or(0) + track.codec_params.start_ts;
    let length = time_base.calc_time(duration);
    let length = length.seconds as f64 + length.frac;
    let mut decoder = CODEC_REGISTRY
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(io::Error::other)?;
    let mut output: Option<Box<dyn output::AudioOutput>> = None;
    let mut output_spec: Option<symphonia::core::audio::SignalSpec> = None;
    let mut output_capacity = 0;
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
        if controls.progress_interval != progress_interval {
            progress_interval = controls.progress_interval;
            force_report = progress_interval.is_some();
        }
        let pause_changed = controls.paused != paused;
        paused = controls.paused;
        if pause_changed {
            if let Some(output) = &mut output {
                output.set_paused(paused).map_err(io::Error::other)?;
            }
            force_report = true;
        }
        if let Some((target, next_revision)) = controls.seek {
            revision = next_revision;
            match format.seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: target,
                    track_id: Some(track_id),
                },
            ) {
                Ok(seeked) => {
                    decoder.reset();
                    if let Some(output) = &mut output {
                        output.discard_queued();
                    }
                    seek_target = Some(seeked.required_ts);
                    let position = time_base.calc_time(seeked.required_ts);
                    position_floor = position.seconds as f64 + position.frac;
                    time.position = position_floor;
                    draining = false;
                }
                Err(err) => eprintln!("error seeking: {err}"),
            }
            force_report = true;
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
        if draining {
            if let (Some(output), Some(spec)) = (&output, output_spec) {
                let pending = output.pending_frames();
                if pending > 0 {
                    control.wait(
                        version,
                        Duration::from_secs_f64(pending as f64 / spec.rate as f64),
                    );
                    continue;
                }
            }
            break;
        }
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(err))
                if err.kind() == io::ErrorKind::UnexpectedEof =>
            {
                draining = true;
                continue;
            }
            Err(err) => return Err(io::Error::other(err)),
        };
        if packet.track_id() != track_id {
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
        let skip_frames = match seek_target {
            Some(target) if packet.ts() < target => {
                let skip = time_base.calc_time(target - packet.ts());
                ((skip.seconds as f64 + skip.frac) * decoded.spec().rate as f64).round() as usize
            }
            _ => 0,
        };
        if skip_frames >= decoded.frames() {
            continue;
        }
        seek_target = None;
        let frames = decoded.frames();
        let source_rate = decoded.spec().rate;
        let mut spec = *decoded.spec();
        spec.rate = (spec.rate as f32 * controls.speed).round() as u32;
        let capacity = decoded.capacity() as u64;
        if output_spec != Some(spec) || output_capacity < capacity {
            output = Some(output::try_open(spec, capacity).map_err(io::Error::other)?);
            output_spec = Some(spec);
            output_capacity = capacity;
        }
        if let Some(output) = &mut output {
            if last_volume != controls.volume {
                last_volume = controls.volume;
                let volume = controls.volume.clamp(0.0, 1.0);
                output_volume = 1.0 - (1.0 - volume * volume).sqrt();
            }
            output
                .write(decoded, output_volume, skip_frames)
                .map_err(io::Error::other)?;
            let start = time_base.calc_time(packet.ts());
            time.position =
                (start.seconds as f64 + start.frac + frames as f64 / source_rate as f64
                    - output.pending_frames() as f64 / source_rate as f64)
                    .max(position_floor);
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
