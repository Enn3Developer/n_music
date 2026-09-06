use crate::platform::Platform;
use crate::settings::Settings;
use crate::FileTrack;
use n_event_bus::{job_emits, EventWriter, Job, JobToken, Tagged};
use std::path::PathBuf;
use std::sync::Arc;

pub struct PersistJob {
    pub settings: Settings,
    pub internal_dir: PathBuf,
    pub tracks: Option<Arc<Vec<FileTrack>>>,
}
pub struct Persisted(pub Result<(), String>);
job_emits!(PersistJob => Tagged<Persisted>);

impl Job for PersistJob {
    async fn run(self, tag: u64, writer: EventWriter, _: Option<JobToken>) {
        let result = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            let write = |name: &str, bytes: Vec<u8>| -> std::io::Result<()> {
                let mut file = tempfile::NamedTempFile::new_in(&self.internal_dir)?;
                zstd::stream::copy_encode(bytes.as_slice(), &mut file, 3)?;
                file.persist(self.internal_dir.join(name))
                    .map_err(|error| error.error)?;
                Ok(())
            };
            if let Some(tracks) = self.tracks {
                write(
                    "tracks",
                    bitcode::encode(&crate::settings::TrackCache {
                        path: self.settings.path.clone(),
                        timestamp: self.settings.timestamp,
                        tracks: Arc::unwrap_or_clone(tracks),
                    }),
                )?;
            }
            write("config", bitcode::encode(&self.settings))
        })
        .await;
        writer.emit_tagged(
            tag,
            Persisted(match result {
                Ok(result) => result.map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            }),
        );
    }
}

pub struct DirectoryJob(pub Arc<dyn Platform>);
pub struct DirectoryChosen(pub PathBuf);
job_emits!(DirectoryJob => Tagged<DirectoryChosen>);
impl Job for DirectoryJob {
    async fn run(self, tag: u64, writer: EventWriter, _: Option<JobToken>) {
        self.0.ask_music_dir(tag, writer).await;
    }
}

pub struct OpenLinkJob(pub Arc<dyn Platform>, pub String);
impl Job for OpenLinkJob {
    async fn run(self, _: u64, _: EventWriter, _: Option<JobToken>) {
        self.0.open_link(self.1).await;
    }
}
