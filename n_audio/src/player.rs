use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SendError, Sender};
use std::thread;
use std::thread::JoinHandle;

use crate::music_track::MusicTrack;
use crate::{output, Message, TrackTime, CODEC_REGISTRY};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo};
use symphonia::core::units::Time;

// TODO: update docs

/// The main actor for everything.
/// Using this struct is really easy, just add a file you want to play (be sure of it being an audio file supported by Symphonia) and call `Player::play_next` and you've done everything!
#[derive(Debug)]
pub struct Player {
    is_paused: bool,
    volume: f32,
    playback_speed: f32,
    cached_get_time: Option<TrackTime>,
    thread: Option<JoinHandle<()>>,
    tx: Option<Sender<Message>>,
    rx_t: Option<Receiver<Message>>,
    rx_e: Option<Receiver<Message>>,
}

impl Player {
    /// Instance a new `Player`
    /// `app_name` is a Linux-only feature, but it is required for all platforms nonetheless
    pub fn new(volume: f32, playback_speed: f32) -> Self {
        Player {
            is_paused: false,
            volume,
            playback_speed,
            cached_get_time: None,
            thread: None,
            tx: None,
            rx_t: None,
            rx_e: None,
        }
    }

    /// Pauses the current playing track, if any
    /// It only errors if it can't send the message (so something serious may have happened)
    pub fn pause(&mut self) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send(Message::Pause)?;
            self.is_paused = true;
        }
        Ok(())
    }

    /// Unpauses the current playing track, if any
    /// It only errors if it can't send the message (so something serious may have happened)
    pub fn unpause(&mut self) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send(Message::Play)?;
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

    /// Sets the output volume
    /// It only errors if it can't send the message (so something serious may have happened)
    pub fn set_volume(&mut self, volume: f32) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send(Message::Volume(volume))?;
        }
        self.volume = volume;
        Ok(())
    }

    /// Sets the output volume
    /// It only errors if it can't send the message (so something serious may have happened)
    pub fn set_playback_speed(&mut self, playback_speed: f32) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send(Message::PlaybackSpeed(playback_speed))?;
            self.playback_speed = playback_speed;
        }
        Ok(())
    }

    /// Seeks to the set timestamp
    /// Be aware that if the timestamp isn't valid the track thread will panic
    /// It only errors if it can't send the message (so something serious may have happened)
    pub fn seek_to(&self, secs: u64, frac: f64) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            if let Some(current_duration) = &self.cached_get_time {
                let min_value = if secs as f64 + frac
                    <= current_duration.dur_secs as f64 + current_duration.dur_frac
                {
                    Time {
                        seconds: secs,
                        frac,
                    }
                } else {
                    Time {
                        seconds: current_duration.dur_secs,
                        frac: current_duration.dur_frac,
                    }
                };
                tx.send(Message::Seek(min_value))?;
            }
        }
        Ok(())
    }

    /// Returns the timestamp that was lastly sent by the track thread
    pub fn get_time(&mut self) -> Option<TrackTime> {
        let mut last = None;

        if let Some(rx_t) = &self.rx_t {
            while let Ok(message) = rx_t.try_recv() {
                if let Message::Time(time) = message {
                    last = Some(time);
                }
            }
        }

        self.cached_get_time = last.clone();
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
    pub fn end_current(&self) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send(Message::Exit)?;
        }
        Ok(())
    }

    /// Plays a certain track given its file path
    pub fn play_from_path<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn Error>>
    where
        P: AsRef<OsStr>,
    {
        let music_track = MusicTrack::new(path)?;
        self.play(music_track.get_format());

        Ok(())
    }

    /// Plays a certain track
    pub fn play_from_track(&mut self, track: &MusicTrack) {
        self.play(track.get_format());
    }

    /// Plays a certain track given its format
    pub fn play(&mut self, format: Box<dyn FormatReader>) {
        let volume = self.volume;
        let playback_speed = self.playback_speed;

        let (tx, rx) = mpsc::channel();
        let (tx_t, rx_t) = mpsc::channel();
        let (tx_e, rx_e) = mpsc::channel();

        let thread =
            thread::spawn(move || Self::thread_fn(format, rx, tx_t, tx_e, volume, playback_speed));

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
        let time_base = track.codec_params.time_base.unwrap();
        let duration = track
            .codec_params
            .n_frames
            .map(|frames| track.codec_params.start_ts + frames)
            .unwrap();

        let mut decoder = CODEC_REGISTRY
            .make(&track.codec_params, &DecoderOptions::default())
            .expect("Can't load decoder");
        let mut audio_output = None;

        let mut spec = None;
        let mut dur = None;

        // Vars used to control audio output
        let mut is_paused = false;
        let mut exit = false;

        loop {
            let recv = |rx: &Receiver<Message>| {
                if is_paused {
                    rx.recv().ok()
                } else {
                    rx.try_recv().ok()
                }
            };
            if let Some(message) = recv(&rx) {
                match message {
                    Message::Play => is_paused = false,
                    Message::Pause => is_paused = true,
                    Message::Volume(v) => volume = v,
                    Message::PlaybackSpeed(speed) => playback_speed = speed,
                    Message::Exit => {
                        exit = true;
                        break;
                    }
                    Message::Seek(time) => {
                        if let Err(err) = format.seek(
                            SeekMode::Coarse,
                            SeekTo::Time {
                                time,
                                track_id: Some(track_id),
                            },
                        ) {
                            println!("error seeking");
                            if !err.to_string().contains("end of stream") {
                                eprintln!(
                                    "Couldn't seek to position {}+{}\nError: {}",
                                    time.seconds, time.frac, err
                                );
                            } else {
                                break;
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
                let ts_time = time_base.calc_time(packet.ts());
                let dur_time = time_base.calc_time(duration);
                if let Err(err) = tx_t.send(Message::Time(TrackTime {
                    ts_secs: ts_time.seconds,
                    ts_frac: ts_time.frac,
                    dur_secs: dur_time.seconds,
                    dur_frac: dur_time.frac,
                })) {
                    if let Ok(message) = rx.try_recv() {
                        if let Message::Exit = message {
                            exit = true;
                            break;
                        }
                        if exit {
                            break;
                        } else {
                            panic!("Can't send Time message: {}", err);
                        }
                    }
                }

                match decoder.decode(&packet) {
                    Ok(decoded) => {
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
                            audio_output.write(decoded, volume).unwrap()
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
