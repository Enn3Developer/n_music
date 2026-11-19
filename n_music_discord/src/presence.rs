//! The thread that keeps Discord's activity in step with playback.

use crate::cover::Covers;
use discord_rich_presence::activity::{
    Activity, ActivityType, Assets, StatusDisplayType, Timestamps,
};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The id of N Music's application in Discord's developer portal.
const CLIENT_ID: &str = "1564753905072611378";
/// The app's logo, uploaded to the application's art assets under this name.
const LOGO: &str = "logo";
/// How long to wait before trying Discord again when it is not running.
const RETRY: Duration = Duration::from_secs(15);

pub(crate) enum Command {
    Track {
        title: String,
        artist: String,
        album: Option<String>,
        length: f64,
    },
    Playing(bool),
    Position {
        position: f64,
        length: f64,
    },
    Enabled(bool),
    Quit,
}

#[derive(Default)]
struct Track {
    title: String,
    artist: String,
    album: Option<String>,
    length: f64,
    cover: Option<String>,
}

struct Presence {
    client: DiscordIpcClient,
    connected: bool,
    enabled: bool,
    track: Option<Track>,
    playing: bool,
    /// Seconds into the track at `at`.
    position: f64,
    at: Instant,
    covers: Covers,
}

pub(crate) fn run(receiver: flume::Receiver<Command>, enabled: bool) {
    let mut presence = Presence {
        client: DiscordIpcClient::new(CLIENT_ID),
        connected: false,
        enabled,
        track: None,
        playing: false,
        position: 0.0,
        at: Instant::now(),
        covers: Covers::default(),
    };
    loop {
        match receiver.recv_timeout(RETRY) {
            Ok(Command::Quit) | Err(flume::RecvTimeoutError::Disconnected) => break,
            Ok(command) => {
                presence.apply(command);
                // Show only the last of a burst of changes, such as skipping through tracks.
                for command in receiver.try_iter() {
                    if let Command::Quit = command {
                        presence.close();
                        return;
                    }
                    presence.apply(command);
                }
                presence.push();
            }
            // Discord was not running: maybe it is now.
            Err(flume::RecvTimeoutError::Timeout) if !presence.connected => presence.push(),
            Err(flume::RecvTimeoutError::Timeout) => {}
        }
    }
    presence.close();
}

impl Presence {
    fn apply(&mut self, command: Command) {
        match command {
            Command::Track {
                title,
                artist,
                album,
                length,
            } => {
                let same = self
                    .track
                    .as_ref()
                    .is_some_and(|track| track.title == title && track.artist == artist);
                // Sent again when the track's metadata loads: keep its position then.
                if !same {
                    self.position = 0.0;
                    self.at = Instant::now();
                }
                self.track = Some(Track {
                    title,
                    artist,
                    album,
                    length,
                    cover: None,
                });
            }
            Command::Playing(playing) => {
                self.position = self.position();
                self.at = Instant::now();
                self.playing = playing;
            }
            Command::Position { position, length } => {
                self.position = position;
                self.at = Instant::now();
                if let Some(track) = &mut self.track {
                    if length > 0.0 {
                        track.length = length;
                    }
                }
            }
            Command::Enabled(enabled) => self.enabled = enabled,
            Command::Quit => {}
        }
    }

    /// Seconds into the track now.
    fn position(&self) -> f64 {
        if self.playing {
            self.position + self.at.elapsed().as_secs_f64()
        } else {
            self.position
        }
    }

    fn connect(&mut self) -> bool {
        if !self.connected {
            match self.client.connect() {
                Ok(()) => {
                    log::info!("Connected to Discord");
                    self.connected = true;
                }
                Err(error) => log::debug!("Discord is not reachable: {error}"),
            }
        }
        self.connected
    }

    /// Sends the current state to Discord.
    fn push(&mut self) {
        if !self.enabled {
            if self.connected {
                let _ = self.client.clear_activity();
            }
            return;
        }
        if !self.connect() {
            return;
        }
        let Some(track) = &mut self.track else {
            let _ = self.client.clear_activity();
            return;
        };
        if track.cover.is_none() {
            track.cover = self
                .covers
                .find(&track.artist, track.album.as_deref(), &track.title);
        }
        let track = self.track.as_ref().unwrap();
        let position = self.position();

        let large_text = track.album.as_deref().unwrap_or(&track.title);
        let mut assets = Assets::new()
            .large_image(track.cover.as_deref().unwrap_or(LOGO))
            .large_text(field(large_text));
        if !self.playing {
            assets = assets.small_image(LOGO).small_text("Paused");
        }
        let mut activity = Activity::new()
            .activity_type(ActivityType::Listening)
            .status_display_type(StatusDisplayType::Details)
            .details(field(&track.title))
            .assets(assets);
        if !track.artist.is_empty() {
            activity = activity.state(field(&track.artist));
        }
        // Paused tracks show no time: Discord would keep counting.
        if self.playing {
            let start = unix_millis() - (position * 1000.0) as i64;
            let mut timestamps = Timestamps::new().start(start);
            if track.length > 0.0 {
                timestamps = timestamps.end(start + (track.length * 1000.0) as i64);
            }
            activity = activity.timestamps(timestamps);
        }

        if let Err(error) = self.client.set_activity(activity) {
            log::warn!("Could not update the Discord presence: {error}");
            // Most likely Discord was closed: connect again next time.
            let _ = self.client.close();
            self.connected = false;
        }
    }

    fn close(&mut self) {
        if self.connected {
            let _ = self.client.clear_activity();
            let _ = self.client.close();
        }
    }
}

/// Discord takes 2 to 128 characters in a text field.
fn field(text: &str) -> String {
    let mut text: String = text.chars().take(128).collect();
    while text.chars().count() < 2 {
        text.push('\u{200b}');
    }
    text
}

fn unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_millis() as i64)
        .unwrap_or_default()
}
