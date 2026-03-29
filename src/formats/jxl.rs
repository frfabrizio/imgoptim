//! JPEG XL decode support (input-only).

use std::io::Cursor;

use crate::error::{ImgOptimError, ResultError};
use crate::formats::convert::{RawColor, RawImage};

use jxl_oxide::JxlImage;

pub(crate) fn decode_to_raw(input: &[u8]) -> ResultError<RawImage> {
    let image = JxlImage::from_reader(Cursor::new(input))
        .map_err(|e| ImgOptimError::Processing(format!("JXL decode failed: {e}")))?;
    let frame = image
        .render_frame(0)
        .map_err(|e| ImgOptimError::Processing(format!("JXL render failed: {e}")))?;

    let width = frame.width();
    let height = frame.height();
    let pixels = frame.to_rgba8();

    Ok(RawImage {
        width,
        height,
        color: RawColor::Rgba8,
        pixels,
    })
}
