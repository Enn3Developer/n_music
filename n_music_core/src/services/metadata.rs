use crate::music_track::MusicTrack;
use crate::services::image::get_image_squared;
use crate::Metadata;
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
    fn run(self, tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        let path = self.path;
        let metadata = match MusicTrack::new(path.to_string_lossy().to_string())
            .and_then(|track| track.get_meta())
        {
            Ok(metadata) => metadata,
            Err(error) => {
                log::warn!("Metadata job {tag} failed for {}: {error}", path.display());
                writer.emit_tagged(tag, MetadataLoaded(Mutex::new(None)));
                return;
            }
        };
        let image = get_image_squared(&path, 0, 0);
        let cover = image.and_then(|image| {
            let file = NamedTempFile::new()
                .inspect_err(|error| {
                    log::warn!(
                        "Could not create a temporary cover file for {}: {error}",
                        path.display()
                    )
                })
                .ok()?;
            image
                .save_to(file.path(), ImageFormat::PNG)
                .inspect_err(|error| {
                    log::warn!("Could not save cover art for {}: {error:?}", path.display())
                })
                .ok()?;
            Some(file)
        });
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
