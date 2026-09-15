use crate::messages::{ScanFinished, TrackMetadataLoaded, TracksEnumerated};
use crate::services::image::get_image_squared;
use crate::settings::Settings;
use crate::FileTrack;
use crate::music_track::MusicTrack;
use crate::{remove_ext, strip_absolute_path};
use n_event_bus::{job_emits, EventWriter, Job, JobToken, Tagged};
use rand::prelude::SliceRandom;
use rand::rng;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;

fn enumerate_audio_files(path: &str) -> Vec<String> {
    let mut names = vec![];

    if let Ok(dir) = std::fs::read_dir(path) {
        for file in dir.flatten() {
            if !file.file_type().map(|t| t.is_file()).unwrap_or(false) {
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
    pub settings: Settings,
    pub internal_dir: PathBuf,
    pub check_cache: bool,
}

job_emits!(ScanJob => Tagged<TracksEnumerated>, Tagged<TrackMetadataLoaded>, Tagged<ScanFinished>);

impl Job for ScanJob {
    fn run(self, tag: u64, writer: EventWriter, token: Option<JobToken>) {
        let path = self.settings.path.clone();
        let names = enumerate_audio_files(&path);
        let len = names.len();

        let internal_dir = self.internal_dir;
        let timestamp = self.settings.timestamp().ok();
        let check_timestamp = self.settings.check_timestamp();
        let file_tracks = self.settings.read_tracks(internal_dir);
        let is_cached = check_timestamp
            && !file_tracks.is_empty()
            && self.check_cache
            && names.iter().all(|name| {
                file_tracks
                    .iter()
                    .any(|track| track.path == remove_ext(name))
            });
        println!("check timestamp: {check_timestamp}; is cached: {is_cached}");

        let mut tracks = Vec::with_capacity(len);
        for name in names.iter() {
            if is_cached {
                let track_without_ext = remove_ext(name);
                if let Some(file_track) = file_tracks
                    .iter()
                    .find(|file_track| file_track.path == track_without_ext)
                {
                    tracks.push(file_track.clone());
                }
            } else {
                tracks.push(FileTrack {
                    path: remove_ext(name),
                    title: remove_ext(name),
                    artist: String::new(),
                    length: 0.0,
                    image: vec![],
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
            writer.emit_tagged(
                tag,
                ScanFinished {
                    tracks: None,
                    path,
                    timestamp,
                },
            );
            return;
        }

        let concurrency = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
            .min(4);
        let queue = Mutex::new(names.into_iter().enumerate());
        let (tx, rx) = std::sync::mpsc::channel::<FileTrack>();
        std::thread::scope(|scope| {
            for _ in 0..concurrency {
                let tx = tx.clone();
                let queue = &queue;
                let writer = &writer;
                let path = &path;
                let token = &token;
                scope.spawn(move || loop {
                    if token.as_ref().is_some_and(JobToken::is_cancelled) {
                        break;
                    }
                    let Some((index, name)) = queue.lock().unwrap().next() else {
                        break;
                    };
                    let track_path = Path::new(path).join(&name);
                    if let Some(file_track) = load_metadata(name, track_path) {
                        writer.emit_tagged(
                            tag,
                            TrackMetadataLoaded {
                                index,
                                track: file_track.clone(),
                            },
                        );
                        let _ = tx.send(file_track);
                    }
                });
            }
        });
        drop(tx);
        let mut file_tracks: Vec<FileTrack> = rx.iter().collect();
        file_tracks.shrink_to_fit();
        writer.emit_tagged(
            tag,
            ScanFinished {
                tracks: Some(Arc::new(file_tracks)),
                path,
                timestamp,
            },
        );
    }
}

fn load_metadata(name: String, path: PathBuf) -> Option<FileTrack> {
    let track = MusicTrack::new(path.to_string_lossy().to_string()).ok()?;
    let meta = track.get_meta().ok()?;
    let image = get_image_squared(path, 128, 128);

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
