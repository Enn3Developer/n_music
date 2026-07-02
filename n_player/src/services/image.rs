use multitag::data::Picture;
use multitag::Tag;
use rimage::codecs::webp::WebPDecoder;
use rimage::operations::resize::{FilterType, ResizeAlg};
use std::fmt::Debug;
use std::io::Cursor;
use std::path::Path;
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_image::image::Image;
use zune_image::traits::{DecoderTrait, OperationsTrait};
use zune_imageprocs::crop::Crop;

pub async fn get_image_squared<P: AsRef<Path> + Debug + Send + 'static>(
    path: P,
    width: usize,
    height: usize,
) -> Option<Image> {
    if let Ok(image) = tokio::task::spawn_blocking(move || get_image(path)).await {
        if !image.is_empty() {
            let zune_image =
                if let Ok(image) = Image::read(ZCursor::new(&image), DecoderOptions::new_fast()) {
                    Some(image)
                } else if let Ok(mut webp_decoder) = WebPDecoder::try_new(Cursor::new(&image)) {
                    if let Ok(image) = webp_decoder.decode() {
                        Some(image)
                    } else {
                        None
                    }
                } else {
                    None
                };

            if let Some(mut zune_image) = zune_image {
                zune_image.convert_color(ColorSpace::RGB).unwrap();
                let (w, h) = zune_image.dimensions();
                let mut size = w;
                if w != h {
                    let difference = w.abs_diff(h);
                    let min = w.min(h);
                    size = min;
                    let is_height = h < w;
                    let x = if is_height { difference / 2 } else { 0 };
                    let y = if !is_height { difference / 2 } else { 0 };
                    tokio::task::block_in_place(|| {
                        Crop::new(min, min, x, y).execute(&mut zune_image).unwrap()
                    });
                }
                tokio::task::block_in_place(|| {
                    rimage::operations::resize::Resize::new(
                        if width == 0 { size } else { width },
                        if height == 0 { size } else { height },
                        ResizeAlg::Convolution(FilterType::Hamming),
                    )
                    .execute(&mut zune_image)
                    .unwrap()
                });
                Some(zune_image)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub fn get_image<P: AsRef<Path> + Debug>(path: P) -> Vec<u8> {
    if let Ok(tag) = Tag::read_from_path(path.as_ref()) {
        if let Some(album) = tag.get_album_info() {
            if let Some(cover) = album.cover {
                return cover.data;
            } else {
                if let Tag::OpusTag { inner } = tag {
                    let cover = inner.pictures().first().cloned().map(Picture::from);
                    if let Some(cover) = cover {
                        return cover.data;
                    }
                } else if let Tag::Id3Tag { inner } = tag {
                    let cover = inner.pictures().next().cloned().map(Picture::from);
                    if let Some(cover) = cover {
                        return cover.data;
                    }
                } else {
                    eprintln!("not an opus or mp3 tag {path:?}");
                }
            }
        } else {
            eprintln!("no album for {path:?}");
        }
    }

    vec![]
}
