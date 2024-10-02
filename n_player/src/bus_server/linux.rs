use crate::bus_server::{BusServer, Property};
use mpris_server::zbus::zvariant::ObjectPath;
use mpris_server::{PlaybackStatus, Server, Time};

impl<T> BusServer for Server<T> {
    async fn properties_changed<P: IntoIterator<Item = Property>>(
        &self,
        properties: P,
    ) -> Result<(), String> {
        self.properties_changed(properties.into_iter().map(|p| match p {
            Property::Playing(playing) => mpris_server::Property::PlaybackStatus(if playing {
                PlaybackStatus::Playing
            } else {
                PlaybackStatus::Paused
            }),
            Property::Metadata(metadata) => {
                let mut meta = mpris_server::Metadata::new();

                meta.set_title(metadata.title);
                meta.set_artist(metadata.artists);
                meta.set_length(Some(Time::from_secs(metadata.length as i64)));
                meta.set_art_url(metadata.image_path);
                meta.set_trackid(Some(ObjectPath::from_string_unchecked(metadata.id)));

                mpris_server::Property::Metadata(meta)
            }
            Property::Volume(volume) => mpris_server::Property::Volume(volume),
        }))
        .await
    }
}
