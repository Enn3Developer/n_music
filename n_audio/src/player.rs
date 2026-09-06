use crate::music_track::MusicTrack;
use crate::{output, Message, TrackTime, CODEC_REGISTRY};
use flume::{Receiver, SendError, Sender};
use std::ffi::OsStr;
use std::path::Path;
use std::thread::JoinHandle;
use std::{io, thread};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo};
use symphonia::core::units::Time;
// TODO: update docs

/// The main actor for everything.
///
/// Using this struct is really easy, just add a file you want to play (be sure of it being an audio file supported by Symphonia or it being an opus file) and call `Player::play` and you've done everything!
#[derive(Debug)]
pub struct Player {
    is_paused: bool,
    volume: f32,
    playback_speed: f32,
    cached_get_time: Option<TrackTime>,
    seek_revision: u64,
    thread: Option<JoinHandle<()>>,
    tx: Option<Sender<Message>>,
    rx_t: Option<Receiver<Message>>,
    rx_e: Option<Receiver<Message>>,
}

impl Player {
    /// Instance a new `Player`
    pub fn new(volume: f32, playback_speed: f32) -> Self {
        Player {
            is_paused: false,
            volume,
            playback_speed,
            cached_get_time: None,
            seek_revision: 0,
            thread: None,
            tx: None,
            rx_t: None,
            rx_e: None,
        }
    }

    /// Pauses the current playing track, if any
    /// It only errors if it can't send the message (so something serious may have happened)
    pub async fn pause(&mut self) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send_async(Message::Pause).await?;
            self.is_paused = true;
        }
        Ok(())
    }

    /// Unpauses the current playing track, if any
    /// It only errors if it can't send the message (so something serious may have happened)
    pub async fn unpause(&mut self) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send_async(Message::Play).await?;
            self.is_paused = false;
        }
        Ok(())
    }

    /// Returns whether the current track is paused
    /// It'll always return `false` if there isn't any track playing
    pub fn is_paused(&self) -> bool {
        if let Some(_tx) = &self.tx {
            return self.is_paused;
        }
        false
    }

    pub fn get_volume(&self) -> f32 {
        self.volume
    }

    /// Sets the output volume
    /// It only errors if it can't send the message (so something serious may have happened)
    pub async fn set_volume(&mut self, volume: f32) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send_async(Message::Volume(volume)).await?;
        }
        self.volume = volume;
        Ok(())
    }

    /// Sets the playback speed
    /// It only errors if it can't send the message (so something serious may have happened)
    pub async fn set_playback_speed(
        &mut self,
        playback_speed: f32,
    ) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send_async(Message::PlaybackSpeed(playback_speed))
                .await?;
            self.playback_speed = playback_speed;
        }
        Ok(())
    }

    /// Seeks to the set timestamp
    /// Be aware that if the timestamp isn't valid the track thread will panic
    /// It only errors if it can't send the message (so something serious may have happened)
    pub async fn seek_to(&mut self, seconds: u64, mut frac: f64) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            if seconds == 0 && frac == 0.0 {
                frac = 0.01;
            }

            let time = Time { seconds, frac };

            let revision = self.seek_revision.wrapping_add(1);
            tx.send_async(Message::Seek(time, revision)).await?;
            self.seek_revision = revision;
            self.cached_get_time = None;
        }
        Ok(())
    }

    /// Returns the timestamp that was lastly sent by the track thread
    pub fn get_time(&mut self) -> Option<TrackTime> {
        let mut last = self.cached_get_time;

        if let Some(rx_t) = &self.rx_t {
            while let Ok(message) = rx_t.try_recv() {
                if let Message::Time(time, revision) = message {
                    if revision == self.seek_revision {
                        last = Some(time);
                    }
                }
            }
        }

        self.cached_get_time = last;
        last
    }

    /// Returns whether the track thread has sent `Message::End`, thus stopping the execution by itself
    /// This will return `false` if you called `Player::end_current` beforehand
    pub fn has_ended(&self) -> bool {
        if let Some(rx_e) = &self.rx_e {
            while let Ok(message) = rx_e.try_recv() {
                if let Message::End = message {
                    return true;
                }
            }
        }
        false
    }

    /// Returns whether if any track is playing
    /// Note that this function doesn't check if the track is paused or not
    pub fn is_playing(&self) -> bool {
        self.thread.is_some()
    }

    /// Ends the current track playing, if any
    /// It only errors if it can't send the message (so something serious may have happened)
    pub async fn end_current(&mut self) -> Result<(), SendError<Message>> {
        let result = if let Some(tx) = self.tx.take() {
            tx.send_async(Message::Exit).await
        } else {
            Ok(())
        };
        if let Some(thread) = self.thread.take() {
            let _ = tokio::task::spawn_blocking(move || thread.join()).await;
        }
        self.rx_t = None;
        self.rx_e = None;
        self.cached_get_time = None;
        self.is_paused = false;
        result
    }

    /// Plays a certain track given its file path
    pub fn play_from_path<P: AsRef<Path> + AsRef<OsStr> + Clone + Into<String>>(
        &mut self,
        path: P,
    ) -> io::Result<()> {
        let music_track = MusicTrack::new(path)?;
        self.play(music_track.get_format()?);

        Ok(())
    }

    /// Plays a certain track
    pub fn play_from_track(&mut self, track: &MusicTrack) -> io::Result<()> {
        self.play(track.get_format()?);
        Ok(())
    }

    /// Plays a certain track given its format
    pub fn play(&mut self, format: Box<dyn FormatReader>) {
        let volume = self.volume;
        let playback_speed = self.playback_speed;

        let (tx, rx) = flume::unbounded();
        let (tx_t, rx_t) = flume::unbounded();
        let (tx_e, rx_e) = flume::unbounded();

        let thread =
            thread::spawn(move || Self::thread_fn(format, rx, tx_t, tx_e, volume, playback_speed));

        self.is_paused = false;
        self.seek_revision = 0;
        self.cached_get_time = None;
        self.rx_e = Some(rx_e);
        self.rx_t = Some(rx_t);
        self.tx = Some(tx);
        self.thread = Some(thread);
    }

    fn thread_fn(
        mut format: Box<dyn FormatReader>,
        rx: Receiver<Message>,
        tx_t: Sender<Message>,
        tx_e: Sender<Message>,
        mut volume: f32,
        mut playback_speed: f32,
    ) {
        // Vars used for audio output
        let track = format.default_track().expect("Can't load tracks");
        let track_id = track.id;
        let time_base = track.codec_params.time_base;
        let duration = track
            .codec_params
            .n_frames
            .map(|frames| track.codec_params.start_ts + frames)
            .unwrap();

        let mut decoder = CODEC_REGISTRY
            .make(&track.codec_params, &DecoderOptions::default())
            .expect("Can't load decoder");
        let mut audio_output: Option<Box<dyn output::AudioOutput>> = None;

        let mut spec = None;
        let mut dur = None;

        // Vars used to control audio output
        let mut is_paused = false;
        let mut exit = false;
        let mut seek_target = None;
        let mut seek_revision = 0;
        let mut reported_time = TrackTime::default();
        let mut position_floor = 0.0;

        loop {
            if let Some(message) = if is_paused {
                rx.recv().ok()
            } else {
                rx.try_recv().ok()
            } {
                match message {
                    Message::Play => is_paused = false,
                    Message::Pause => is_paused = true,
                    // from: https://stackoverflow.com/a/1165198
                    Message::Volume(v) => volume = 1.0 - (1.0 - (v * v)).sqrt(),
                    Message::PlaybackSpeed(speed) => playback_speed = speed,
                    Message::Exit => {
                        exit = true;
                        break;
                    }
                    Message::Seek(time, revision) => {
                        seek_revision = revision;
                        match format.seek(
                            SeekMode::Accurate,
                            SeekTo::Time {
                                time,
                                track_id: Some(track_id),
                            },
                        ) {
                            Ok(seeked) => {
                                decoder.reset();
                                if let Some(output) = &mut audio_output {
                                    output.discard_queued();
                                }
                                seek_target = Some(seeked.required_ts);
                                if let Some(time_base) = time_base {
                                    let position = time_base.calc_time(seeked.required_ts);
                                    let length = time_base.calc_time(duration);
                                    position_floor = position.seconds as f64 + position.frac;
                                    reported_time = TrackTime {
                                        position: position_floor,
                                        length: length.seconds as f64 + length.frac,
                                    };
                                    if is_paused {
                                        let _ =
                                            tx_t.send(Message::Time(reported_time, seek_revision));
                                    }
                                }
                            }
                            Err(err) => {
                                let _ = tx_t.send(Message::Time(reported_time, seek_revision));
                                if err.to_string().contains("end of stream") {
                                    break;
                                }
                                eprintln!(
                                    "Couldn't seek to position {}+{}\nError: {}",
                                    time.seconds, time.frac, err
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }

            if exit {
                break;
            }

            if !is_paused {
                let packet = match format.next_packet() {
                    Ok(packet) => packet,
                    Err(_err) => {
                        break;
                    }
                };

                if packet.track_id() != track_id {
                    continue;
                }

                while !format.metadata().is_latest() {
                    format.metadata().pop();
                }

                match decoder.decode(&packet) {
                    Ok(decoded) => {
                        let skip_frames = match (seek_target, time_base) {
                            (Some(target), Some(base)) if packet.ts() < target => {
                                let skip = base.calc_time(target - packet.ts());
                                ((skip.seconds as f64 + skip.frac) * decoded.spec().rate as f64)
                                    .round() as usize
                            }
                            _ => 0,
                        };
                        if skip_frames >= decoded.frames() {
                            continue;
                        }
                        seek_target = None;
                        let frames = decoded.frames();
                        let sample_rate = decoded.spec().rate;
                        if audio_output.is_none() {
                            let mut tmp_spec = *decoded.spec();
                            tmp_spec.rate = (tmp_spec.rate as f32 * playback_speed).round() as u32;
                            spec = Some(tmp_spec);
                            dur = Some(decoded.capacity() as u64);
                            audio_output =
                                Some(output::try_open(spec.unwrap(), dur.unwrap()).unwrap());
                        } else {
                            let mut new_spec = *decoded.spec();
                            new_spec.rate = (new_spec.rate as f32 * playback_speed).round() as u32;
                            let new_dur = decoded.capacity() as u64;
                            let mut changed = false;
                            if new_spec != spec.unwrap() {
                                spec = Some(new_spec);
                                changed = true;
                            }
                            if new_dur != dur.unwrap() {
                                dur = Some(new_dur);
                                changed = true
                            }
                            if changed {
                                audio_output =
                                    Some(output::try_open(spec.unwrap(), dur.unwrap()).unwrap());
                            }
                        }

                        if let Some(audio_output) = &mut audio_output {
                            audio_output.write(decoded, volume, skip_frames).unwrap();
                            if let Some(base) = time_base {
                                let start = base.calc_time(packet.ts());
                                let length = base.calc_time(duration);
                                let position = start.seconds as f64
                                    + start.frac
                                    + frames as f64 / sample_rate as f64
                                    - audio_output.pending_frames() as f64 / sample_rate as f64;
                                reported_time = TrackTime {
                                    position: position.max(position_floor),
                                    length: length.seconds as f64 + length.frac,
                                };
                                let _ = tx_t.send(Message::Time(reported_time, seek_revision));
                            }
                        }
                    }
                    Err(symphonia::core::errors::Error::DecodeError(err)) => {
                        eprintln!("Decode error: {}", err);
                    }
                    Err(err) => {
                        eprintln!("Error has occurred in decoding packet: {}", err);
                        break;
                    }
                }
            }
        }
        if !exit {
            tx_e.send(Message::End).expect("Can't send End message");
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new(1.0, 1.0)
    }
}
