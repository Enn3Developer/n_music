use crate::state::{Backend, Change, Emit, LoopStatus, MediaEvent, State};
use mpris_server::zbus::fdo;
use mpris_server::{
    zbus, LoopStatus as MprisLoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface,
    Property, RootInterface, Server, Time, TrackId, Volume,
};
use std::sync::{Arc, RwLock};

pub(crate) fn new_controls(emit: Emit, state: Arc<RwLock<State>>) -> Option<Box<dyn Backend>> {
    Some(Box::new(Mpris::new(emit, state)?))
}

pub(crate) struct Mpris {
    notifier: Notifier,
}

impl Mpris {
    pub(crate) fn new(emit: Emit, state: Arc<RwLock<State>>) -> Option<Self> {
        let server = match zbus::block_on(Server::new(
            "n_music",
            Adapter {
                emit: emit.clone(),
                state: state.clone(),
            },
        )) {
            Ok(server) => server,
            Err(_) => {
                let name = format!("n_music_{}", std::process::id());
                zbus::block_on(Server::new(&name, Adapter { emit, state })).ok()?
            }
        };
        let server = Arc::new(server);
        let (property_tx, property_rx) = flume::unbounded::<Property>();

        // zbus drives the connection on its own internal executor thread; this
        // worker only has to serialize `properties_changed` calls off the event
        // bus thread, coalescing bursts of property updates.
        std::thread::Builder::new()
            .name(String::from("n_music_mpris"))
            .spawn(move || {
                while let Ok(first) = property_rx.recv() {
                    let mut properties = vec![first];
                    while let Ok(property) = property_rx.try_recv() {
                        let kind = std::mem::discriminant(&property);
                        properties.retain(|old| std::mem::discriminant(old) != kind);
                        properties.push(property);
                    }
                    if let Err(error) = zbus::block_on(server.properties_changed(properties)) {
                        eprintln!("error notifying mpris: {error}");
                    }
                }
            })
            .ok()?;

        Some(Self {
            notifier: Notifier {
                properties: property_tx,
            },
        })
    }
}

impl Backend for Mpris {
    fn update(&self, state: &State, change: Change) {
        let property = match change {
            Change::Playback => Property::PlaybackStatus(if state.playing {
                PlaybackStatus::Playing
            } else {
                PlaybackStatus::Paused
            }),
            Change::Volume => Property::Volume(state.volume),
            Change::Loop => Property::LoopStatus(match state.loop_status {
                LoopStatus::Playlist => MprisLoopStatus::Playlist,
                LoopStatus::File => MprisLoopStatus::Track,
            }),
            Change::Metadata => Property::Metadata(metadata(state)),
            Change::Position { .. } => return,
        };
        self.notifier.notify(property);
    }
}

/// Forwards property changes to the backend thread, so `zbus` never sees
/// out-of-order updates.
struct Notifier {
    properties: flume::Sender<Property>,
}

impl Notifier {
    fn notify(&self, property: Property) {
        let _ = self.properties.send(property);
    }
}

fn metadata(state: &State) -> Metadata {
    let mut metadata = Metadata::new();
    metadata.set_title(Some(state.title.clone()));
    metadata.set_artist(if state.artist.is_empty() {
        None
    } else {
        Some(vec![state.artist.clone()])
    });
    metadata.set_length(Some(Time::from_secs(state.length as i64)));
    metadata.set_art_url(
        state
            .cover
            .as_ref()
            .map(|path| format!("file://{}", path.display())),
    );
    metadata.set_trackid(Some(
        mpris_server::zbus::zvariant::ObjectPath::from_static_str_unchecked("/n_music"),
    ));
    metadata
}

struct Adapter {
    emit: Emit,
    state: Arc<RwLock<State>>,
}

impl RootInterface for Adapter {
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

impl PlayerInterface for Adapter {
    async fn next(&self) -> fdo::Result<()> {
        (self.emit)(MediaEvent::Next);
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        (self.emit)(MediaEvent::Previous);
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        (self.emit)(MediaEvent::Pause);
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        (self.emit)(MediaEvent::TogglePause);
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        (self.emit)(MediaEvent::Play);
        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        (self.emit)(MediaEvent::SeekRelative(offset.as_millis() as f64 / 1000.0));
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, position: Time) -> fdo::Result<()> {
        (self.emit)(MediaEvent::SeekAbsolute(
            position.as_millis() as f64 / 1000.0,
        ));
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

    async fn loop_status(&self) -> fdo::Result<MprisLoopStatus> {
        match self.state.read().unwrap().loop_status {
            LoopStatus::Playlist => Ok(MprisLoopStatus::Playlist),
            LoopStatus::File => Ok(MprisLoopStatus::Track),
        }
    }

    async fn set_loop_status(&self, loop_status: MprisLoopStatus) -> zbus::Result<()> {
        (self.emit)(MediaEvent::SetLoopStatus(match loop_status {
            MprisLoopStatus::None => LoopStatus::Playlist,
            MprisLoopStatus::Track => LoopStatus::File,
            MprisLoopStatus::Playlist => LoopStatus::Playlist,
        }));
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
        Ok(metadata(&self.state.read().unwrap()))
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(self.state.read().unwrap().volume)
    }

    async fn set_volume(&self, volume: Volume) -> zbus::Result<()> {
        (self.emit)(MediaEvent::SetVolume(volume));
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
