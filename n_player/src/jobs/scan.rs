use crate::messages::{ScanFinished, TrackMetadataLoaded, TracksEnumerated};
use crate::platform::Platform;
use crate::services::image::get_image_squared;
use crate::settings::Settings;
use crate::{FileTrack, TrackData};
use n_audio::music_track::MusicTrack;
use n_audio::{remove_ext, strip_absolute_path};
use n_event_bus::{job_emits, EventWriter, Job, JobToken, Tagged};
use rand::prelude::SliceRandom;
use rand::rng;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinSet;

async fn enumerate_audio_files(path: &str) -> Vec<String> {
    let mut names = vec![];

    if let Ok(mut dir) = tokio::fs::read_dir(path).await {
        while let Ok(Some(file)) = dir.next_entry().await {
            if !file.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let Ok(Some(mime)) = infer::get_from_path(file.path()) else {
                continue;
            };
            if !mime.mime_type().contains("audio") {
                continue;
            }

            names.push(strip_absolute_path(
                file.path().to_string_lossy().to_string(),
            ));
        }
    }
    names.shuffle(&mut rng());
    names.shrink_to_fit();
    names
}

pub struct ScanJob {
    pub settings: Arc<RwLock<Settings>>,
    pub platform: Arc<dyn Platform>,
    pub check_cache: bool,
}

job_emits!(ScanJob => Tagged<TracksEnumerated>, Tagged<TrackMetadataLoaded>, Tagged<ScanFinished>);

impl Job for ScanJob {
    async fn run(self, tag: u64, writer: EventWriter, token: Option<JobToken>) {
        let path = self.settings.read().await.path.clone();
        let names = enumerate_audio_files(&path).await;
        let len = names.len();

        let internal_dir = self.platform.internal_dir().await;
        let (check_timestamp, file_tracks) = {
            let settings = self.settings.read().await;
            (
                settings.check_timestamp().await,
                settings.read_tracks(internal_dir.clone()).await,
            )
        };
        let is_cached = check_timestamp && !file_tracks.is_empty() && self.check_cache;
        println!("check timestamp: {check_timestamp}; is cached: {is_cached}");

        let mut tracks = Vec::with_capacity(len);
        for (i, name) in names.iter().enumerate() {
            if is_cached {
                let track_without_ext = remove_ext(name);
                if let Some(file_track) = file_tracks
                    .iter()
                    .find(|file_track| file_track.path == track_without_ext)
                {
                    let mut track: TrackData = file_track.clone().into();
                    track.index = i as i32;
                    tracks.push(track);
                }
            } else {
                tracks.push(TrackData {
                    artist: Default::default(),
                    cover: Default::default(),
                    time: Default::default(),
                    title: remove_ext(name).into(),
                    index: i as i32,
                    visible: true,
                });
            }
        }
        tracks.shrink_to_fit();
        writer.emit_tagged(
            tag,
            TracksEnumerated {
                path: path.clone(),
                names: names.clone(),
                tracks,
            },
        );

        if is_cached {
            writer.emit_tagged(tag, ScanFinished { tracks: None });
            return;
        }

        self.settings
            .read()
            .await
            .clear_tracks(internal_dir.clone())
            .await;

        let semaphore = Arc::new(Semaphore::new(num_cpus::get() * 4));
        let mut tasks = JoinSet::new();
        for (index, name) in names.into_iter().enumerate() {
            if token.as_ref().map(JobToken::is_cancelled).unwrap_or(false) {
                return;
            }
            let semaphore = semaphore.clone();
            let writer = writer.clone();
            let track_path = Path::new(&path).join(&name);
            tasks.spawn(async move {
                let _permit = semaphore.acquire_owned().await.ok()?;
                let file_track = load_metadata(index, name, track_path).await?;
                writer.emit_tagged(
                    tag,
                    TrackMetadataLoaded {
                        index,
                        track: file_track.clone(),
                    },
                );
                Some(file_track)
            });
        }

        let mut file_tracks = Vec::with_capacity(len);
        while let Some(result) = tasks.join_next().await {
            if let Ok(Some(file_track)) = result {
                file_tracks.push(file_track);
            }
        }
        writer.emit_tagged(
            tag,
            ScanFinished {
                tracks: Some(file_tracks),
            },
        );
    }
}

async fn load_metadata(_index: usize, name: String, path: PathBuf) -> Option<FileTrack> {
    let track = MusicTrack::new(path.to_string_lossy().to_string()).ok()?;
    let meta = tokio::task::spawn_blocking(move || track.get_meta())
        .await
        .ok()?
        .ok()?;
    let image = get_image_squared(path, 128, 128).await;

    Some(FileTrack {
        path: remove_ext(name),
        title: meta.title,
        artist: meta.artist,
        length: meta.time.length,
        image: image
            .map(|i| i.flatten_to_u8()[0].clone())
            .unwrap_or(vec![]),
    })
}
