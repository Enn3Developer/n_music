use crate::messages::{
    LoopStatusChanged, Pause, Play, PlayNext, PlayPrevious, PlaybackChanged, PositionChanged, Seek,
    SetLoopStatus, SetVolume, TogglePause, TrackChanged, VolumeChanged,
};
use crate::services::metadata::{MetadataJob, MetadataLoaded, MetadataLoader};
use n_event_bus::{Ctx, EventWriter, Handle, Outbox, Registrar, Subscriber, Tagged};
use std::any::Any;
use std::sync::{Arc, RwLock};
use tempfile::NamedTempFile;

pub struct MprisState {
    playing: bool,
    volume: f64,
    position: f64,
    loop_status: n_audio::queue::LoopStatus,
    metadata: mpris_server::Metadata,
}

impl Default for MprisState {
    fn default() -> Self {
        Self {
            playing: false,
            volume: n_audio::queue::QueuePlayer::default().get_volume() as f64,
            position: 0.0,
            loop_status: n_audio::queue::LoopStatus::default(),
            metadata: mpris_server::Metadata::new(),
        }
    }
}

pub struct MprisBridge {
    server: Arc<mpris_server::Server<MprisAdapter>>,
    state: Arc<RwLock<MprisState>>,
    tmp: Option<NamedTempFile>,
    metadata_loader: MetadataLoader,
}

impl MprisBridge {
    pub async fn new(writer: EventWriter) -> Option<Self> {
        let state = Arc::new(RwLock::new(MprisState::default()));
        let adapter = |writer: EventWriter| MprisAdapter {
            writer,
            state: state.clone(),
        };
        let server = match mpris_server::Server::new("n_music", adapter(writer.clone())).await {
            Ok(server) => server,
            Err(_) => {
                let name = format!("n_music_{}", std::process::id());
                mpris_server::Server::new(&name, adapter(writer))
                    .await
                    .ok()?
            }
        };

        Some(Self {
            server: Arc::new(server),
            state,
            tmp: None,
            metadata_loader: MetadataLoader::default(),
        })
    }

    fn notify(&self, property: mpris_server::Property) {
        let server = self.server.clone();
        tokio::spawn(async move {
            if let Err(err) = server.properties_changed([property]).await {
                eprintln!("error notifying mpris: {err}");
            }
        });
    }
}

impl Subscriber for MprisBridge {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<PlaybackChanged>();
        reg.on::<TrackChanged>();
        MetadataJob::subscribe(reg);
        reg.on::<VolumeChanged>();
        reg.on::<PositionChanged>();
        reg.on::<LoopStatusChanged>();
    }
}

impl Handle<PlaybackChanged> for MprisBridge {
    fn handle(&mut self, msg: &PlaybackChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.state.write().unwrap().playing = msg.0;
        self.notify(mpris_server::Property::PlaybackStatus(if msg.0 {
            mpris_server::PlaybackStatus::Playing
        } else {
            mpris_server::PlaybackStatus::Paused
        }));
    }
}

impl Handle<VolumeChanged> for MprisBridge {
    fn handle(&mut self, msg: &VolumeChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.state.write().unwrap().volume = msg.0;
        self.notify(mpris_server::Property::Volume(msg.0));
    }
}

impl Handle<LoopStatusChanged> for MprisBridge {
    fn handle(&mut self, msg: &LoopStatusChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.state.write().unwrap().loop_status = msg.0.clone();
        self.notify(mpris_server::Property::LoopStatus(match msg.0 {
            n_audio::queue::LoopStatus::Playlist => mpris_server::LoopStatus::Playlist,
            n_audio::queue::LoopStatus::File => mpris_server::LoopStatus::Track,
        }));
    }
}

impl Handle<PositionChanged> for MprisBridge {
    fn handle(&mut self, msg: &PositionChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.state.write().unwrap().position = msg.0.position;
    }
}

impl Handle<TrackChanged> for MprisBridge {
    fn handle(&mut self, msg: &TrackChanged, ctx: &Ctx, _out: &mut Outbox) {
        self.metadata_loader.load(msg.path.clone(), ctx);
    }
}

impl Handle<Tagged<MetadataLoaded>> for MprisBridge {
    fn handle(&mut self, msg: &Tagged<MetadataLoaded>, _ctx: &Ctx, _out: &mut Outbox) {
        let Some(loaded) = self.metadata_loader.take(msg) else {
            return;
        };
        let meta = loaded.metadata;
        let mut metadata = mpris_server::Metadata::new();
        metadata.set_title(Some(meta.title));
        metadata.set_artist(if meta.artist.is_empty() {
            None
        } else {
            Some(vec![meta.artist])
        });
        metadata.set_length(Some(mpris_server::Time::from_secs(meta.time.length as i64)));
        metadata.set_art_url(
            loaded
                .cover
                .as_ref()
                .map(|file| format!("file://{}", file.path().display())),
        );
        metadata.set_trackid(Some(
            mpris_server::zbus::zvariant::ObjectPath::from_static_str_unchecked("/n_music"),
        ));
        self.tmp = loaded.cover;
        self.state.write().unwrap().metadata = metadata.clone();
        self.notify(mpris_server::Property::Metadata(metadata));
    }
}

pub struct MprisAdapter {
    writer: EventWriter,
    state: Arc<RwLock<MprisState>>,
}

use mpris_server::zbus::fdo;
use mpris_server::{
    zbus, LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, RootInterface, Time,
    TrackId, Volume,
};

impl RootInterface for MprisAdapter {
    async fn raise(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn quit(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_fullscreen(&self, _fullscreen: bool) -> zbus::Result<()> {
        Ok(())
    }

    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn has_track_list(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn identity(&self) -> fdo::Result<String> {
        Ok(String::from("N Music"))
    }

    async fn desktop_entry(&self) -> fdo::Result<String> {
        Err(fdo::Error::NotSupported(String::from("no entry found")))
    }

    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }

    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }
}

impl PlayerInterface for MprisAdapter {
    async fn next(&self) -> fdo::Result<()> {
        self.writer.emit(PlayNext);
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        self.writer.emit(PlayPrevious);
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        self.writer.emit(Pause);
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        self.writer.emit(TogglePause);
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        self.writer.emit(Play);
        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        self.writer.emit(Seek::Relative(
            offset.as_secs() as f64 + (offset.as_millis() as f64 / 1000.0),
        ));
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, position: Time) -> fdo::Result<()> {
        self.writer
            .emit(Seek::Absolute(position.as_millis() as f64 / 1000.0));
        Ok(())
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Ok(())
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        if self.state.read().unwrap().playing {
            Ok(PlaybackStatus::Playing)
        } else {
            Ok(PlaybackStatus::Paused)
        }
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        match self.state.read().unwrap().loop_status {
            n_audio::queue::LoopStatus::Playlist => Ok(LoopStatus::Playlist),
            n_audio::queue::LoopStatus::File => Ok(LoopStatus::Track),
        }
    }

    async fn set_loop_status(&self, loop_status: LoopStatus) -> zbus::Result<()> {
        let loop_status = match loop_status {
            LoopStatus::None => n_audio::queue::LoopStatus::Playlist,
            LoopStatus::Track => n_audio::queue::LoopStatus::File,
            LoopStatus::Playlist => n_audio::queue::LoopStatus::Playlist,
        };
        self.writer.emit(SetLoopStatus(loop_status));
        Ok(())
    }

    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn set_rate(&self, _rate: PlaybackRate) -> zbus::Result<()> {
        Ok(())
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn set_shuffle(&self, _shuffle: bool) -> zbus::Result<()> {
        Ok(())
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        Ok(self.state.read().unwrap().metadata.clone())
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(self.state.read().unwrap().volume)
    }

    async fn set_volume(&self, volume: Volume) -> zbus::Result<()> {
        self.writer.emit(SetVolume(volume));
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        let position = self.state.read().unwrap().position;
        Ok(Time::from_millis((position * 1000.0).floor() as i64))
    }

    async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}
