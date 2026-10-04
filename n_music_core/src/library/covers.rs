//! Cover thumbnails shared by all tracks, one file per distinct embedded picture.

use rimage::codecs::webp::WebPDecoder;
use rimage::operations::resize::{FilterType, ResizeAlg};
use std::collections::HashSet;
use std::fmt::Display;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_core::options::EncoderOptions;
use zune_image::codecs::ImageFormat;
use zune_image::errors::ImageErrors;
use zune_image::image::Image;
use zune_image::traits::{DecoderTrait, OperationsTrait};
use zune_imageprocs::crop::Crop;

pub const COVER_SIZE: usize = 256;
const JPEG_QUALITY: u8 = 85;
const EXTENSIONS: [&str; 2] = ["jpg", "png"];

pub struct CoverStore {
    dir: PathBuf,
}

impl CoverStore {
    pub fn open(dir: &Path) -> Self {
        let dir = dir.to_path_buf();
        if let Err(error) = std::fs::create_dir_all(&dir) {
            log::warn!("Could not create cover store {}: {error}", dir.display());
        }
        Self { dir }
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// Stores the thumbnail of an embedded cover (`data`, still encoded) and returns its path.
    /// Covers are addressed by the hash of the embedded picture, so a known cover is not decoded
    /// again.
    pub fn store(&self, data: &[u8], locator: impl Display) -> Option<PathBuf> {
        let hash = format!("{:016x}", xxhash_rust::xxh3::xxh3_64(data));
        if let Some(path) = EXTENSIONS
            .iter()
            .map(|ext| format!("{hash}.{ext}"))
            .map(|name| self.path(&name))
            .find(|path| path.is_file())
        {
            return Some(path);
        }

        let image = squared(data, COVER_SIZE, COVER_SIZE, &locator)?;
        let (bytes, ext) = encode(image)
            .inspect_err(|error| log::warn!("Could not encode cover of {locator}: {error:?}"))
            .ok()?;
        let path = self.path(&format!("{hash}.{ext}"));
        // Concurrent scan workers may store the same cover: both write identical bytes.
        super::write_atomic(&path, &bytes)
            .inspect_err(|error| log::warn!("Could not save cover of {locator}: {error}"))
            .ok()?;
        Some(path)
    }

    /// Deletes every stored cover not in `keep` (file names).
    pub fn retain(&self, keep: &HashSet<String>) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let is_cover = Path::new(name)
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| EXTENSIONS.contains(&ext));
            if is_cover && !keep.contains(name) {
                if let Err(error) = std::fs::remove_file(entry.path()) {
                    log::debug!("Could not remove unused cover {name}: {error}");
                }
            }
        }
    }
}

/// Encodes an RGBA thumbnail: JPEG when fully opaque, PNG when it uses transparency.
fn encode(mut image: Image) -> Result<(Vec<u8>, &'static str), ImageErrors> {
    let opaque = image.flatten_to_u8().first().is_some_and(|pixels| {
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[3] == u8::MAX)
    });
    let (format, ext) = if opaque {
        image.convert_color(ColorSpace::RGB)?;
        (ImageFormat::JPEG, "jpg")
    } else {
        (ImageFormat::PNG, "png")
    };
    let mut bytes = vec![];
    format.encode(
        &image,
        EncoderOptions::default().set_quality(JPEG_QUALITY),
        &mut bytes,
    )?;
    Ok((bytes, ext))
}

/// Decodes `data`, crops it to a centered square and resizes it to `width`x`height` RGBA
/// (0 keeps the cropped size).
pub fn squared(data: &[u8], width: usize, height: usize, path: impl Display) -> Option<Image> {
    let mut zune_image =
        if let Ok(image) = Image::read(ZCursor::new(data), DecoderOptions::new_fast()) {
            image
        } else {
            WebPDecoder::try_new(Cursor::new(data))
                .ok()?
                .decode()
                .ok()?
        };
    zune_image
        .convert_color(ColorSpace::RGBA)
        .inspect_err(|error| {
            log::warn!("Could not convert cover art for {path} to RGBA: {error:?}")
        })
        .ok()?;
    let (w, h) = zune_image.dimensions();
    let mut size = w;
    if w != h {
        let difference = w.abs_diff(h);
        let min = w.min(h);
        size = min;
        let is_height = h < w;
        let x = if is_height { difference / 2 } else { 0 };
        let y = if !is_height { difference / 2 } else { 0 };
        Crop::new(min, min, x, y)
            .execute(&mut zune_image)
            .inspect_err(|error| log::warn!("Could not crop cover art for {path}: {error:?}"))
            .ok()?;
    }
    rimage::operations::resize::Resize::new(
        if width == 0 { size } else { width },
        if height == 0 { size } else { height },
        ResizeAlg::Convolution(FilterType::Hamming),
    )
    .execute(&mut zune_image)
    .inspect_err(|error| log::warn!("Could not resize cover art for {path}: {error:?}"))
    .ok()?;
    Some(zune_image)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(alpha: u8) -> Image {
        let pixels: Vec<u8> = (0..16 * 16).flat_map(|_| [200, 100, 50, alpha]).collect();
        Image::from_u8(&pixels, 16, 16, ColorSpace::RGBA)
    }

    #[test]
    fn opaque_covers_are_jpeg() {
        let (bytes, ext) = encode(rgba(u8::MAX)).unwrap();
        assert_eq!(ext, "jpg");
        assert!(bytes.starts_with(&[0xFF, 0xD8]));
    }

    #[test]
    fn transparent_covers_are_png() {
        let (bytes, ext) = encode(rgba(128)).unwrap();
        assert_eq!(ext, "png");
        assert!(bytes.starts_with(b"\x89PNG"));
    }
}
