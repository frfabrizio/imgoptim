//! TIFF decode support (input-only).

use crate::error::{ImgOptimError, ResultError};
use crate::formats::convert::{RawColor, RawImage};

use image::{DynamicImage, GenericImageView, ImageFormat};

pub(crate) fn decode_to_raw(input: &[u8]) -> ResultError<RawImage> {
    let img = image::load_from_memory_with_format(input, ImageFormat::Tiff)
        .map_err(|e| ImgOptimError::Processing(format!("TIFF decode failed: {e}")))?;
    Ok(dynamic_to_raw(img))
}

fn dynamic_to_raw(img: DynamicImage) -> RawImage {
    let (width, height) = img.dimensions();

    match img {
        DynamicImage::ImageLuma8(inner) => RawImage {
            width,
            height,
            color: RawColor::L8,
            pixels: inner.into_raw(),
        },
        DynamicImage::ImageRgb8(inner) => RawImage {
            width,
            height,
            color: RawColor::Rgb8,
            pixels: inner.into_raw(),
        },
        DynamicImage::ImageRgba8(inner) => RawImage {
            width,
            height,
            color: RawColor::Rgba8,
            pixels: inner.into_raw(),
        },
        DynamicImage::ImageLumaA8(inner) => {
            let rgba = DynamicImage::ImageLumaA8(inner).to_rgba8().into_raw();
            RawImage {
                width,
                height,
                color: RawColor::Rgba8,
                pixels: rgba,
            }
        }
        other => {
            let rgba = other.to_rgba8().into_raw();
            RawImage {
                width,
                height,
                color: RawColor::Rgba8,
                pixels: rgba,
            }
        }
    }
}
