use crate::get_image;
use crate::platform::Platform;
use crate::runner::Runner;
use n_audio::music_track::MusicTrack;
use n_audio::remove_ext;
use std::io::{Seek, Write};
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::sync::{Mutex, RwLock};

#[cfg(target_os = "linux")]
pub mod linux;

pub enum Property {
    Playing(bool),
    Metadata(Metadata),
    Volume(f64),
}

pub struct Metadata {
    pub title: Option<String>,
    pub artists: Option<Vec<String>>,
    pub length: f64,
    pub id: String,
    pub image_path: Option<String>,
}

pub async fn run<P: Platform>(
    platform: Arc<Mutex<P>>,
    runner: Arc<RwLock<Runner>>,
    mut tmp: NamedTempFile,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(250));
    let mut properties = vec![];
    let mut playback = false;
    let mut volume = 1.0;
    let mut index = runner.read().await.index();
    let path = runner.read().await.path();

    loop {
        interval.tick().await;
        let guard = runner.read().await;

        if playback != guard.playback() {
            playback = guard.playback();
            properties.push(Property::Playing(playback));
        }
        if volume != guard.volume() {
            volume = guard.volume();
            properties.push(Property::Volume(volume))
        }

        if index != guard.index() {
            index = guard.index();
            let track_name = &guard.current_track().await.unwrap();
            let mut path_buf = PathBuf::new();
            path_buf.push(&path);
            path_buf.push(track_name);
            let track = MusicTrack::new(path_buf.to_str().unwrap())
                .expect("can't get track for currently playing song");
            let meta = track.get_meta();
            let image = tokio::task::spawn_blocking(|| get_image(path_buf))
                .await
                .unwrap();
            let image_path = if image.is_empty() {
                None
            } else {
                tmp.rewind().expect("can't rewind tmp file");
                tmp.write_all(&image)
                    .expect("can't write image data to tmp file");
                Some(tmp.path().to_str().unwrap().to_string())
            };
            if let Ok(meta) = meta {
                properties.push(Property::Metadata(Metadata {
                    id: String::from("/n_music"),
                    title: Some(if !meta.title.is_empty() {
                        meta.title
                    } else {
                        remove_ext(track_name)
                    }),
                    artists: if meta.artist.is_empty() {
                        None
                    } else {
                        Some(vec![meta.artist])
                    },
                    length: meta.time.length,
                    image_path,
                }));
            }
        }

        if !properties.is_empty() {
            platform
                .lock()
                .await
                .properties_changed(mem::take(&mut properties));
        }
    }
}
