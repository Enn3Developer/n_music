use flume::Receiver;
use n_audio::queue::QueuePlayer;
use n_audio::TrackTime;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub async fn run(runner: Arc<RwLock<Runner>>) {
    let mut interval = tokio::time::interval(Duration::from_millis(250));
    loop {
        interval.tick().await;
        runner.write().await.update().await;
    }
}

pub enum RunnerMessage {
    PlayNext,
    PlayPrevious,
    TogglePause,
    Pause,
    Play,
    SetVolume(f64),
    PlayTrack(String),
    Seek(Seek),
}

pub enum Seek {
    Absolute(f64),
    Relative(f64),
}

pub struct Runner {
    player: QueuePlayer,
    rx: Receiver<RunnerMessage>,
    current_time: TrackTime,
}

impl Runner {
    pub fn new(rx: Receiver<RunnerMessage>, player: QueuePlayer) -> Self {
        Self {
            player,
            rx,
            current_time: TrackTime::default(),
        }
    }

    pub async fn update(&mut self) {
        while let Ok(message) = self.rx.try_recv() {
            self.parse_command(message).await;
        }

        self.current_time = self.player.get_time().unwrap();

        if self.player.has_ended() {
            self.player.play_next();
        }
    }

    async fn parse_command(&mut self, message: RunnerMessage) {
        match message {
            RunnerMessage::PlayNext => {
                self.player.end_current().await.unwrap();
                self.player.play_next();
            }
            RunnerMessage::PlayPrevious => {
                if self.current_time.pos_secs < 2 {
                    self.player.seek_to(0, 0.0).await.unwrap();
                } else {
                    self.player.end_current().await.unwrap();
                    self.player.play_previous();
                }
            }
            RunnerMessage::TogglePause => {
                if self.player.is_paused() {
                    self.player.unpause().await.unwrap();
                } else {
                    self.player.pause().await.unwrap();
                }
                if !self.player.is_playing() {
                    self.player.play_next();
                }
            }
            RunnerMessage::Pause => {
                self.player.pause().await.unwrap();
            }
            RunnerMessage::Play => {
                self.player.unpause().await.unwrap();
            }
            RunnerMessage::SetVolume(volume) => {
                self.player.set_volume(volume as f32).await.unwrap();
            }
            RunnerMessage::PlayTrack(name) => {
                self.player
                    .play_index(self.player.get_index_from_track_name(&name).unwrap());
            }
            RunnerMessage::Seek(seek) => {
                let seek = match seek {
                    Seek::Absolute(value) => value,
                    Seek::Relative(value) => {
                        self.current_time.pos_secs as f64 + self.current_time.pos_frac + value
                    }
                };
                self.player
                    .seek_to(seek.trunc() as u64, seek.fract())
                    .await
                    .unwrap();
            }
        }
    }

    pub fn playback(&self) -> bool {
        self.player.is_paused() && self.player.is_playing()
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

    pub fn queue(&self) -> Vec<String> {
        self.player.queue()
    }

    pub fn index(&self) -> usize {
        self.player.index()
    }
}
