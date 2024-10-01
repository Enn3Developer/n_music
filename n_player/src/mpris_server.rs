use crate::ServerMessage::SetVolume;
use crate::{ClientMessage, ServerMessage};
use flume::{Receiver, Sender};
use mpris_server::zbus::zvariant::ObjectPath;
use mpris_server::RootInterface;
use mpris_server::{
    LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, Time, TrackId, Volume,
};

pub struct MPRISServer {
    tx: Sender<ServerMessage>,
    rx: Receiver<ClientMessage>,
}

impl MPRISServer {
    pub fn new(tx: Sender<ServerMessage>, rx: Receiver<ClientMessage>) -> Self {
        Self { tx, rx }
    }
}

impl RootInterface for MPRISServer {
    async fn raise(&self) -> mpris_server::zbus::fdo::Result<()> {
        Ok(())
    }

    async fn quit(&self) -> mpris_server::zbus::fdo::Result<()> {
        Ok(())
    }

    async fn can_quit(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn fullscreen(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn set_fullscreen(&self, fullscreen: bool) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn can_set_fullscreen(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn can_raise(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn has_track_list(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn identity(&self) -> mpris_server::zbus::fdo::Result<String> {
        Ok(String::from("N Music"))
    }

    async fn desktop_entry(&self) -> mpris_server::zbus::fdo::Result<String> {
        Ok(String::from("N Music.desktop"))
    }

    async fn supported_uri_schemes(&self) -> mpris_server::zbus::fdo::Result<Vec<String>> {
        Ok(vec![])
    }

    async fn supported_mime_types(&self) -> mpris_server::zbus::fdo::Result<Vec<String>> {
        Ok(vec![])
    }
}

impl PlayerInterface for MPRISServer {
    async fn next(&self) -> mpris_server::zbus::fdo::Result<()> {
        self.tx.send(ServerMessage::PlayNext).unwrap();
        Ok(())
    }

    async fn previous(&self) -> mpris_server::zbus::fdo::Result<()> {
        self.tx.send(ServerMessage::PlayPrevious).unwrap();
        Ok(())
    }

    async fn pause(&self) -> mpris_server::zbus::fdo::Result<()> {
        Ok(())
    }

    async fn play_pause(&self) -> mpris_server::zbus::fdo::Result<()> {
        self.tx.send(ServerMessage::TogglePause).unwrap();
        Ok(())
    }

    async fn stop(&self) -> mpris_server::zbus::fdo::Result<()> {
        Ok(())
    }

    async fn play(&self) -> mpris_server::zbus::fdo::Result<()> {
        Ok(())
    }

    async fn seek(&self, offset: Time) -> mpris_server::zbus::fdo::Result<()> {
        Ok(())
    }

    async fn set_position(
        &self,
        track_id: TrackId,
        position: Time,
    ) -> mpris_server::zbus::fdo::Result<()> {
        Ok(())
    }

    async fn open_uri(&self, uri: String) -> mpris_server::zbus::fdo::Result<()> {
        Ok(())
    }

    async fn playback_status(&self) -> mpris_server::zbus::fdo::Result<PlaybackStatus> {
        self.tx.send(ServerMessage::AskPlayback).unwrap();
        while let Ok(message) = self.rx.recv() {
            if let ClientMessage::Playback(playback) = message {
                return if playback {
                    Ok(PlaybackStatus::Playing)
                } else {
                    Ok(PlaybackStatus::Paused)
                };
            }
        }

        Ok(PlaybackStatus::Paused)
    }

    async fn loop_status(&self) -> mpris_server::zbus::fdo::Result<LoopStatus> {
        Ok(LoopStatus::Playlist)
    }

    async fn set_loop_status(&self, loop_status: LoopStatus) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn rate(&self) -> mpris_server::zbus::fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn set_rate(&self, rate: PlaybackRate) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn shuffle(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(true)
    }

    async fn set_shuffle(&self, shuffle: bool) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn metadata(&self) -> mpris_server::zbus::fdo::Result<Metadata> {
        let mut metadata = Metadata::new();
        self.tx.send(ServerMessage::AskMetadata).unwrap();

        metadata.set_trackid(Some(ObjectPath::from_static_str("/empty").unwrap()));

        while let Ok(message) = self.rx.recv() {
            if let ClientMessage::Metadata(title, artist, time, path) = message {
                metadata.set_title(title);
                metadata.set_artist(artist);
                metadata.set_length(Some(Time::from_secs(time as i64)));
                metadata.set_trackid(Some(ObjectPath::from_string_unchecked(path)));
            }
        }

        Ok(metadata)
    }

    async fn volume(&self) -> mpris_server::zbus::fdo::Result<Volume> {
        self.tx.send(ServerMessage::AskVolume).unwrap();
        while let Ok(message) = self.rx.recv() {
            if let ClientMessage::Volume(volume) = message {
                return Ok(volume);
            }
        }

        Ok(1.0)
    }

    async fn set_volume(&self, volume: Volume) -> mpris_server::zbus::Result<()> {
        self.tx.send(SetVolume(volume)).unwrap();
        Ok(())
    }

    async fn position(&self) -> mpris_server::zbus::fdo::Result<Time> {
        Ok(Time::from_millis(0))
    }

    async fn minimum_rate(&self) -> mpris_server::zbus::fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn maximum_rate(&self) -> mpris_server::zbus::fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn can_go_next(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(true)
    }

    async fn can_go_previous(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(true)
    }

    async fn can_play(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(true)
    }

    async fn can_pause(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(true)
    }

    async fn can_seek(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }

    async fn can_control(&self) -> mpris_server::zbus::fdo::Result<bool> {
        Ok(false)
    }
}
