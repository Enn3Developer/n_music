use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SendError, Sender};
use std::thread;
use std::thread::JoinHandle;

#[cfg(feature = "shuffle")]
use rand::seq::SliceRandom;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::{SeekMode, SeekTo};
use symphonia::core::units::Time;

use crate::music_track::MusicTrack;
use crate::{output, Message, NError, TrackTime};

/// The main actor for everything.
/// Using this struct is really easy, just add a file you want to play (be sure of it being an audio file supported by Symphonia) and call `Player::play_next` and you've done everything!
pub struct Player {
    tracks: Vec<MusicTrack>,
    index: usize,
    index_playing: usize,
    is_first: bool,
    is_paused: bool,
    // Only used for Pulse Audio
    app_name: String,
    cached_get_time: Option<TrackTime>,
    thread: Option<JoinHandle<()>>,
    tx: Option<Sender<Message>>,
    rx_t: Option<Receiver<Message>>,
    rx_e: Option<Receiver<Message>>,
}

impl Player {
    /// Instance a new `Player`
    /// `app_name` is a Linux-only feature but it is required for all platforms nonetheless
    pub fn new(app_name: String) -> Self {
        Player {
            tracks: vec![],
            index: 0,
            index_playing: 0,
            is_first: true,
            is_paused: false,
            app_name,
            cached_get_time: None,
            thread: None,
            tx: None,
            rx_t: None,
            rx_e: None,
        }
    }

    pub fn add_to_queue<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn Error>>
    where
        P: AsRef<OsStr>,
    {
        let track = MusicTrack::new(path)?;
        self.tracks.push(track);
        Ok(())
    }

    /// Shuffles the `tracks` vector using the `rand` crate
    #[cfg(feature = "shuffle")]
    pub fn shuffle(&mut self) {
        self.tracks.shuffle(&mut rand::thread_rng());
    }

    /// Returns the `tracks` vector
    pub fn tracks(&self) -> &Vec<MusicTrack> {
        &self.tracks
    }

    /// Clears the `tracks` vector and sets both `index` and `index_playing` to 0
    pub fn clear_tracks(&mut self) {
        self.tracks.clear();
        self.index = 0;
        self.index_playing = 0;
    }

    /// Removes a specific track from the `tracks` vector
    pub fn remove_track(&mut self, index: usize) {
        self.tracks.remove(index);
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
    pub fn set_volume(&self, volume: f32) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send(Message::Volume(volume))?;
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

    /// This will return the current playing track's name
    /// Note that this function will return a value even if there isn't any track playing
    /// Note that this function returns the track's name at index 0 by default
    pub fn get_current_track_name(&self) -> String {
        return self
            .tracks
            .get(self.index_playing)
            .unwrap()
            .name()
            .to_string(); // <- Why calling to_string to circumvent the borrow checker?!
    }

    /// Returns the index of the current track playing
    /// Note that this function will return a value even if there isn't any track playing
    /// Note that this function returns 0 by default
    pub fn get_index_current_track(&self) -> usize {
        self.index_playing
    }

    /// Returns the duration for a certain track
    /// `panic!`s if you passed an invalid index
    pub fn get_duration_for_track(&self, index: usize) -> TrackTime {
        let format = self.tracks.get(index).unwrap().get_format();
        let track = format.default_track().expect("Can't load tracks");
        let time_base = track.codec_params.time_base.unwrap();
        let duration = track
            .codec_params
            .n_frames
            .map(|frames| track.codec_params.start_ts + frames)
            .unwrap();
        let time = time_base.calc_time(duration);
        TrackTime {
            ts_secs: 0,
            ts_frac: 0.0,
            dur_secs: time.seconds,
            dur_frac: time.frac,
        }
    }

    /// Returns the name for a certain track
    /// `panic!`s if you passed an invalid index
    pub fn get_index_from_track_name(&self, name: &str) -> Result<usize, NError> {
        for i in 0..self.tracks.len() {
            if self.tracks.get(i).unwrap().name() == name {
                return Ok(i);
            }
        }
        Err(NError::NoTrack)
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

    /// Plays the track next in queue
    /// If it already played all the tracks it will restart from 0
    pub fn play_next(&mut self) {
        if !self.is_first {
            self.index += 1;

            if self.index == self.tracks.len() {
                self.index = 0;
            }
        } else {
            self.is_first = false;
        }

        // Here is false because the new index is already set
        self.play(self.index, false);
    }

    /// Plays the track that was previous in line
    /// If the current `index` is 0 it'll wrap to the last track in queue
    pub fn play_previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.tracks.len() - 1;
        }

        // Here is false because the new index is already set
        self.play(self.index, false);
    }

    /// Plays a certain track
    /// If `set_new_index` is true it'll set `index` to the given index
    /// `panic!`s if the index is invalid
    pub fn play(&mut self, index: usize, set_new_index: bool) {
        let app_name = self.app_name.clone();

        self.index_playing = index;

        let format = self
            .tracks
            .get(index)
            .expect("No audio file added to the player")
            .get_format();

        if set_new_index {
            self.index = index;
        }

        let (tx, rx) = mpsc::channel();
        let (tx_t, rx_t) = mpsc::channel();
        let (tx_e, rx_e) = mpsc::channel();

        let thread = thread::spawn(move || {
            // Vars used for audio output
            let mut format = format;

            let track = format.default_track().expect("Can't load tracks");
            let track_id = track.id;
            let time_base = track.codec_params.time_base.unwrap();
            let duration = track
                .codec_params
                .n_frames
                .map(|frames| track.codec_params.start_ts + frames)
                .unwrap();

            let mut decoder = symphonia::default::get_codecs()
                .make(&track.codec_params, &DecoderOptions::default())
                .expect("Can't load decoder");
            let mut audio_output = None;

            let mut spec = None;
            let mut dur = None;

            // Vars used to control audio output
            let mut is_paused = false;
            let mut exit = false;

            let mut volume = 1.0;

            loop {
                while let Ok(message) = rx.try_recv() {
                    match message {
                        Message::Play => is_paused = false,
                        Message::Pause => is_paused = true,
                        Message::Volume(v) => volume = v,
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
                        while let Ok(message) = rx.try_recv() {
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
                                spec = Some(*decoded.spec());
                                dur = Some(decoded.capacity() as u64);
                                audio_output = Some(
                                    output::try_open(spec.unwrap(), dur.unwrap(), &app_name)
                                        .unwrap(),
                                );
                            } else {
                                let new_spec = *decoded.spec();
                                let new_dur = decoded.capacity() as u64;

                                if new_spec != spec.unwrap() {
                                    spec = Some(new_spec);
                                }

                                if new_dur != dur.unwrap() {
                                    dur = Some(new_dur);
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
            format
                .seek(
                    SeekMode::Coarse,
                    SeekTo::Time {
                        time: Time {
                            seconds: 0,
                            frac: 0.0,
                        },
                        track_id: None,
                    },
                )
                .unwrap();
        });

        self.rx_e = Some(rx_e);
        self.rx_t = Some(rx_t);
        self.tx = Some(tx);
        self.thread = Some(thread);
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new("N Audio".to_string())
    }
}
