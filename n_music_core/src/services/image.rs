use multitag::data::Picture;
use multitag::Tag;
use rimage::codecs::webp::WebPDecoder;
use rimage::operations::resize::{FilterType, ResizeAlg};
use std::fmt::Display;
use std::io::Cursor;
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_image::image::Image;
use zune_image::traits::{DecoderTrait, OperationsTrait};
use zune_imageprocs::crop::Crop;

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

/// The embedded cover of a tag read by multitag, for files Symphonia finds no picture in.
pub fn tag_cover(tag: Tag) -> Option<Vec<u8>> {
    if let Some(cover) = tag.get_album_info().and_then(|album| album.cover) {
        return Some(cover.data);
    }
    let cover = match tag {
        Tag::OpusTag { inner } => inner.pictures().first().cloned().map(Picture::from),
        Tag::Id3Tag { inner } => inner.pictures().next().cloned().map(Picture::from),
        _ => None,
    };
    cover.map(|cover| cover.data)
}
