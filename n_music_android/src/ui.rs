use crate::TrackData;
use n_music_core::{FileTrack, Theme};
use slint::private_unstable_api::re_exports::ColorScheme;
use slint::SharedPixelBuffer;

unsafe impl Send for TrackData {}
unsafe impl Sync for TrackData {}

pub fn color_scheme(theme: Theme) -> ColorScheme {
    match theme {
        Theme::System => ColorScheme::Unknown,
        Theme::Light => ColorScheme::Light,
        Theme::Dark => ColorScheme::Dark,
    }
}

pub fn to_track_data(mut value: FileTrack, index: i32) -> TrackData {
    value.artist.shrink_to_fit();
    value.title.shrink_to_fit();
    value.image.shrink_to_fit();
    TrackData {
        artist: value.artist.into(),
        cover: match value.image.len() {
            // Older library caches contain RGB thumbnails; new scans retain alpha.
            len if len == 128 * 128 * 3 => {
                slint::Image::from_rgb8(SharedPixelBuffer::clone_from_slice(&value.image, 128, 128))
            }
            len if len == 128 * 128 * 4 => slint::Image::from_rgba8(
                SharedPixelBuffer::clone_from_slice(&value.image, 128, 128),
            ),
            _ => Default::default(),
        },
        index,
        time: format!(
            "{:02}:{:02}",
            (value.length / 60.0).floor() as u64,
            value.length.floor() as u64 % 60
        )
        .into(),
        title: value.title.into(),
        visible: true,
    }
}
