use std::any::Any;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SendError, Sender};
use std::thread;
use std::thread::JoinHandle;

#[cfg(feature = "shuffle")]
use rand::seq::SliceRandom;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

mod output;

#[derive(Debug)]
pub enum NError {
    NoTrack,
}

pub enum Message {
    Play,
    Pause,
    End,
    Exit,
    Seek(Time),
    Time(TrackTime),
    Volume(f32),
}

pub fn from_path_to_name_without_ext(path: &Path) -> String {
    let split: Vec<String> = path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .split('.')
        .map(String::from)
        .collect();
    split[..split.len() - 1].to_vec().join(".")
}

#[derive(Clone)]
pub struct TrackTime {
    pub ts_secs: u64,
    pub ts_frac: f64,
    pub dur_secs: u64,
    pub dur_frac: f64,
}

pub struct MusicTrack {
    file: File,
    name: String,
    ext: String,
}

impl MusicTrack {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>>
    where
        P: AsRef<OsStr>,
    {
        let path = Path::new(&path);
        let file = File::open(path)?;
        file.metadata().unwrap().file_type().type_id();
        Ok(MusicTrack {
            file,
            name: from_path_to_name_without_ext(path),
            ext: path.extension().unwrap().to_str().unwrap().to_string(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_format(&self) -> Box<dyn FormatReader> {
        let file = self.file.try_clone().expect("Can't copy file");
        let media_stream = MediaSourceStream::new(Box::new(file), std::default::Default::default());
        let mut hint = Hint::new();
        hint.with_extension(self.ext.as_ref());
        let meta_ops = MetadataOptions::default();
        let fmt_ops = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let probed = symphonia::default::get_probe()
            .format(&hint, media_stream, &fmt_ops, &meta_ops)
            .expect("Format not supported");
        probed.format
    }
}

pub struct Player {
    tracks: Vec<MusicTrack>,
    index: usize,
    index_playing: usize,
    is_first: bool,
    is_paused: bool,
    app_name: String,
    // <- Only used for Pulse Audio
    cached_get_time: Option<TrackTime>,
    thread: Option<JoinHandle<()>>,
    tx: Option<Sender<Message>>,
    rx_t: Option<Receiver<Message>>,
    rx_e: Option<Receiver<Message>>,
}

impl Player {
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

    #[cfg(feature = "shuffle")]
    pub fn shuffle(&mut self) {
        self.tracks.shuffle(&mut rand::thread_rng());
    }

    pub fn tracks(&self) -> &Vec<MusicTrack> {
        &self.tracks
    }

    pub fn pause(&mut self) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send(Message::Pause)?;
            self.is_paused = true;
        }
        Ok(())
    }

    pub fn unpause(&mut self) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send(Message::Play)?;
            self.is_paused = false;
        }
        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        if let Some(_tx) = &self.tx {
            return self.is_paused;
        }
        false
    }

    pub fn set_volume(&self, volume: f32) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send(Message::Volume(volume))?;
        }
        Ok(())
    }

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

    pub fn get_current_track_name(&self) -> String {
        return self
            .tracks
            .get(self.index_playing)
            .unwrap()
            .name
            .to_string(); // <- Why calling to_string to circumvent the borrow checker?!
    }

    pub fn get_index_current_track(&self) -> usize {
        self.index_playing
    }

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

    pub fn get_index_from_track_name(&self, name: &str) -> Result<usize, NError> {
        for i in 0..self.tracks.len() {
            if self.tracks.get(i).unwrap().name == name {
                return Ok(i);
            }
        }
        Err(NError::NoTrack)
    }

    pub fn is_playing(&self) -> bool {
        self.thread.is_some()
    }

    pub fn end_current(&self) -> Result<(), SendError<Message>> {
        if let Some(tx) = &self.tx {
            tx.send(Message::Exit)?;
        }
        Ok(())
    }

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

    pub fn play_previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.tracks.len() - 1;
        }

        // Here is false because the new index is already set
        self.play(self.index, false);
    }

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

        self.end_current().unwrap();

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
                                let spec = *decoded.spec();
                                let duration = decoded.capacity() as u64;
                                audio_output =
                                    Some(output::try_open(spec, duration, &app_name).unwrap());
                            } else {
                                // TODO: Check if the audio spec. and duration haven't changed.
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
