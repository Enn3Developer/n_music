use crate::services::image::get_image_squared;
use n_audio::music_track::MusicTrack;
use n_audio::Metadata;
use n_event_bus::{job_emits, Ctx, EventWriter, Job, JobToken, RunningJob, Tagged};
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::NamedTempFile;
use zune_image::codecs::ImageFormat;

pub struct MetadataJob {
    pub path: PathBuf,
}

pub struct TrackMetadata {
    pub metadata: Metadata,
    pub cover: Option<NamedTempFile>,
}

pub struct MetadataLoaded(pub Mutex<Option<TrackMetadata>>);

job_emits!(MetadataJob => Tagged<MetadataLoaded>);

impl Job for MetadataJob {
    async fn run(self, tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        let path = self.path.clone();
        let Ok(Ok(metadata)) = tokio::task::spawn_blocking(move || {
            MusicTrack::new(path.to_string_lossy().to_string())?.get_meta()
        })
        .await
        else {
            writer.emit_tagged(tag, MetadataLoaded(Mutex::new(None)));
            return;
        };
        let image = get_image_squared(self.path, 0, 0).await;
        let cover = tokio::task::spawn_blocking(move || {
            image.and_then(|image| {
                let file = NamedTempFile::new().ok()?;
                image.save_to(file.path(), ImageFormat::PNG).ok()?;
                Some(file)
            })
        })
        .await
        .ok()
        .flatten();
        writer.emit_tagged(
            tag,
            MetadataLoaded(Mutex::new(Some(TrackMetadata { metadata, cover }))),
        );
    }
}

#[derive(Default)]
pub struct MetadataLoader {
    job: Option<RunningJob>,
}

impl MetadataLoader {
    pub fn load(&mut self, path: PathBuf, ctx: &Ctx) {
        self.job = Some(ctx.jobs.spawn_oneshot(MetadataJob { path }));
    }

    pub fn take(&mut self, message: &Tagged<MetadataLoaded>) -> Option<TrackMetadata> {
        let loaded = self.job.as_ref()?.open(message)?;
        let metadata = loaded.0.lock().unwrap().take();
        self.job = None;
        metadata
    }
}
