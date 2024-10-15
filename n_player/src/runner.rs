use flume::Receiver;
use n_audio::queue::QueuePlayer;
use n_audio::TrackTime;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub async fn run(runner: Arc<RwLock<Runner>>, rx: Receiver<RunnerMessage>) {
    let mut interval = tokio::time::interval(Duration::from_millis(500));
    loop {
        tokio::select! {
            _ = interval.tick() => {
                runner.write().await.update().await;
            }
            message = rx.recv_async() => {
                if let Ok(message) = message {
                    runner.write().await.parse_command(message).await;
                }
                runner.write().await.update().await;
            }
        }
    }
}

#[derive(Debug)]
pub enum RunnerMessage {
    PlayNext,
    PlayPrevious,
    TogglePause,
    Pause,
    Play,
    SetVolume(f64),
    PlayTrack(u16),
    Seek(RunnerSeek),
}

#[derive(Debug)]
pub enum RunnerSeek {
    Absolute(f64),
    Relative(f64),
}

pub struct Runner {
    player: QueuePlayer,
    current_time: TrackTime,
}

impl Runner {
    pub fn new(player: QueuePlayer) -> Self {
        Self {
            player,
            current_time: TrackTime::default(),
        }
    }

    pub async fn update(&mut self) {
        if let Some(time) = self.player.get_time() {
            self.current_time = time;
        }

        if self.player.has_ended() {
            if let Err(err) = self.player.play_next().await {
                eprintln!("error happened: {err}");
            }
        }
    }

    async fn parse_command(&mut self, message: RunnerMessage) {
        println!("{message:?}");
        match message {
            RunnerMessage::PlayNext => {
                self.player.end_current().await.unwrap();
                if let Err(err) = self.player.play_next().await {
                    eprintln!("error happened: {err}");
                }
            }
            RunnerMessage::PlayPrevious => {
                if self.current_time.position > 3.0 {
                    self.player.seek_to(0, 0.0).await.unwrap();
                } else {
                    self.player.end_current().await.unwrap();
                    if let Err(err) = self.player.play_previous().await {
                        eprintln!("error happened: {err}");
                    }
                }
            }
            RunnerMessage::TogglePause => {
                if self.player.is_paused() {
                    self.player.unpause().await.unwrap();
                } else {
                    self.player.pause().await.unwrap();
                }
                if !self.player.is_playing() {
                    if let Err(err) = self.player.play_next().await {
                        eprintln!("error happened: {err}");
                    }
                }
            }
            RunnerMessage::Pause => {
                self.player.pause().await.unwrap();
            }
            RunnerMessage::Play => {
                self.player.unpause().await.unwrap();
                if !self.player.is_playing() {
                    if let Err(err) = self.player.play_next().await {
                        eprintln!("error happened: {err}");
                    }
                }
            }
            RunnerMessage::SetVolume(volume) => {
                self.player.set_volume(volume as f32).await.unwrap();
            }
            RunnerMessage::PlayTrack(index) => {
                self.player.end_current().await.unwrap();
                if let Err(err) = self.player.play_index(index).await {
                    eprintln!("error happened: {err}");
                }
            }
            RunnerMessage::Seek(seek) => {
                let seek = match seek {
                    RunnerSeek::Absolute(value) => value,
                    RunnerSeek::Relative(value) => self.current_time.position + value,
                };
                if let Err(e) = self.player.seek_to(seek.trunc() as u64, seek.fract()).await {
                    eprintln!("error happened while asking to seek: {e}");
                }
            }
        }
    }

    pub fn playback(&self) -> bool {
        !self.player.is_paused() && self.player.is_playing()
    }

    pub fn volume(&self) -> f64 {
        self.player.get_volume() as f64
    }

    pub fn time(&self) -> TrackTime {
        self.current_time
    }

    pub fn path(&self) -> String {
        self.player.path()
    }

    pub fn queue(&self) -> Arc<RwLock<BufReader<File>>> {
        self.player.queue()
    }

    pub fn index_map(&self) -> Vec<u64> {
        self.player.index_map()
    }

    pub fn index(&self) -> u16 {
        self.player.index()
    }

    pub fn len(&self) -> usize {
        self.player.len()
    }

    pub fn is_empty(&self) -> bool {
        self.player.is_empty()
    }

    pub async fn get_path_for_file(&self, i: u16) -> Option<PathBuf> {
        self.player.get_path_for_file(i).await
    }

    pub async fn current_track(&self) -> Option<String> {
        self.player.current_track_name().await
    }
}
