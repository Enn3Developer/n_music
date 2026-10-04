use crate::settings::Theme;
use crate::TrackData;
use n_music_core::library::covers::squared;
use n_music_core::{Track, TrackInfo};
use slint::private_unstable_api::re_exports::ColorScheme;
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

unsafe impl Send for TrackData {}
unsafe impl Sync for TrackData {}

pub fn color_scheme(theme: Theme) -> ColorScheme {
    match theme {
        Theme::System => ColorScheme::Unknown,
        Theme::Light => ColorScheme::Light,
        Theme::Dark => ColorScheme::Dark,
    }
}

/// Covers are shown at 64px, decoded at 2x for high-DPI screens.
const THUMBNAIL_SIZE: usize = 128;

pub type CoverBuffer = SharedPixelBuffer<Rgba8Pixel>;

/// Decoded covers by path, so tracks of the same album share one pixel buffer.
#[derive(Default)]
pub struct CoverCache(HashMap<PathBuf, CoverBuffer>);

impl CoverCache {
    pub fn get(&self, path: &Path) -> Option<CoverBuffer> {
        self.0.get(path).cloned()
    }

    /// The cover at `path`, decoding it now if needed.
    pub fn load(&mut self, path: PathBuf) -> Option<CoverBuffer> {
        if let Some(buffer) = self.get(&path) {
            return Some(buffer);
        }
        let buffer = decode_cover(&path)?;
        self.0.insert(path, buffer.clone());
        Some(buffer)
    }

    pub fn insert(&mut self, path: PathBuf, buffer: CoverBuffer) {
        self.0.insert(path, buffer);
    }

    /// Forgets covers that no track uses any more.
    pub fn retain(&mut self, tracks: &[Track]) {
        let used: HashSet<&PathBuf> = tracks.iter().filter_map(|t| t.cover.as_ref()).collect();
        self.0.retain(|path, _| used.contains(path));
    }
}

pub fn decode_cover(path: &Path) -> Option<CoverBuffer> {
    let data = std::fs::read(path)
        .inspect_err(|error| log::debug!("Could not read cover {}: {error}", path.display()))
        .ok()?;
    let image = squared(&data, THUMBNAIL_SIZE, THUMBNAIL_SIZE, path.display())?;
    let pixels = image.flatten_to_u8().into_iter().next()?;
    Some(SharedPixelBuffer::clone_from_slice(
        &pixels,
        THUMBNAIL_SIZE as u32,
        THUMBNAIL_SIZE as u32,
    ))
}

/// `seconds` as `mm:ss`.
pub fn format_time(seconds: f64) -> String {
    format!(
        "{:02}:{:02}",
        (seconds / 60.0).floor() as u64,
        seconds.floor() as u64 % 60
    )
}

/// `cover` is left empty when it is not decoded yet; see [`CoverCache`].
pub fn to_track_data(value: &TrackInfo, index: i32, cover: Option<CoverBuffer>) -> TrackData {
    TrackData {
        artist: value.artist().into(),
        cover: cover.map(slint::Image::from_rgba8).unwrap_or_default(),
        index,
        time: format_time(value.length).into(),
        title: value.title.as_str().into(),
        visible: true,
    }
}
