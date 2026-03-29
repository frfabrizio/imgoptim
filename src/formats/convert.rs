//! Format-agnostic conversion/optimization router (trait-based).
//!
//! - Defines `ImageCodec` and `OptimizeOptions`
//! - Dispatches to format codecs: jpeg/png/webp
//! - Uses crate error type: `ImgOptimError`

use std::fs;
use std::path::Path;

use crate::error::{ImgOptimError, ResultError};
use crate::formats::{self, ImageFormat};

pub type Result<T> = std::result::Result<T, ImgOptimError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawColor {
    L8,
    Rgb8,
    Rgba8,
}

#[derive(Debug, Clone)]
pub(crate) struct RawImage {
    pub width: u32,
    pub height: u32,
    pub color: RawColor,
    pub pixels: Vec<u8>,
}

/// Shared options for all codecs (no clap types here).
#[derive(Debug, Clone, Default)]
pub struct OptimizeOptions {
    // Common
    pub quality: Option<u8>,
    pub max_quality: Option<u8>,

    // JPEG
    pub progressive: bool,

    // PNG
    pub png_level: Option<u8>,
    pub zopfli: bool,
    pub zopfli_iteration_count: Option<u64>,
    pub zopfli_max_block_splits: Option<u16>,
    pub zopfli_timeout_secs: Option<u64>,
    pub zopfli_progress: bool,

    // WebP
    pub webp_lossless: bool,
    pub webp_method: Option<u8>,
}

impl OptimizeOptions {
    fn zopfli_requested(&self) -> bool {
        self.zopfli
            || self.zopfli_iteration_count.is_some()
            || self.zopfli_max_block_splits.is_some()
            || self.zopfli_timeout_secs.is_some()
    }

    pub fn validate(&self, format: ImageFormat) -> ResultError<()> {
        match format {
            ImageFormat::Jpeg => {
                if self.png_level.is_some() || self.zopfli {
                    return Err(ImgOptimError::InvalidArgs(
                        "PNG options are not valid for JPEG".into(),
                    ));
                }
                if self.zopfli_requested() {
                    return Err(ImgOptimError::InvalidArgs(
                        "Zopfli options are not valid for JPEG".into(),
                    ));
                }
                if self.webp_lossless || self.webp_method.is_some() {
                    return Err(ImgOptimError::InvalidArgs(
                        "WebP options are not valid for JPEG".into(),
                    ));
                }
            }
            ImageFormat::Png => {
                if self.quality.is_some() || self.max_quality.is_some() || self.progressive {
                    return Err(ImgOptimError::InvalidArgs(
                        "quality/max_quality/progressive are not applicable to PNG".into(),
                    ));
                }
                if self.webp_lossless || self.webp_method.is_some() {
                    return Err(ImgOptimError::InvalidArgs(
                        "WebP options are not valid for PNG".into(),
                    ));
                }
            }
            ImageFormat::Webp => {
                if self.progressive {
                    return Err(ImgOptimError::InvalidArgs(
                        "progressive is not applicable to WebP".into(),
                    ));
                }
                if self.png_level.is_some() || self.zopfli_requested() {
                    return Err(ImgOptimError::InvalidArgs(
                        "PNG options are not valid for WebP".into(),
                    ));
                }
                if self.webp_lossless && self.quality.is_some() {
                    return Err(ImgOptimError::InvalidArgs(
                        "webp-lossless is incompatible with quality".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Common contract for all format backends.
pub trait ImageCodec {
    const FORMAT: ImageFormat;
    fn optimize(input: &[u8], opts: &OptimizeOptions) -> ResultError<Vec<u8>>;
}

/// Convert/optimize an on-disk file.
pub fn convert_file(
    input: &Path,
    output: &Path,
    format: ImageFormat,
    opts: &OptimizeOptions,
) -> ResultError<()> {
    if !formats::is_built(format) {
        return Err(ImgOptimError::not_built(format));
    }

    opts.validate(format)?;

    let input_bytes = fs::read(input)?;
    let output_bytes = convert_bytes(&input_bytes, format, opts)?;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, output_bytes)?;
    Ok(())
}

/// Convert/optimize from in-memory bytes.
pub fn convert_bytes(
    input: &[u8],
    format: ImageFormat,
    opts: &OptimizeOptions,
) -> ResultError<Vec<u8>> {
    if !formats::is_built(format) {
        return Err(ImgOptimError::not_built(format));
    }

    opts.validate(format)?;

    match format {
        ImageFormat::Jpeg => {
            #[cfg(feature = "jpeg")]
            {
                dispatch::<crate::formats::jpeg::Codec>(input, opts)
            }
            #[cfg(not(feature = "jpeg"))]
            {
                Err(ImgOptimError::not_built(ImageFormat::Jpeg))
            }
        }
        ImageFormat::Png => {
            #[cfg(feature = "png")]
            {
                dispatch::<crate::formats::png::Codec>(input, opts)
            }
            #[cfg(not(feature = "png"))]
            {
                Err(ImgOptimError::not_built(ImageFormat::Png))
            }
        }
        ImageFormat::Webp => {
            #[cfg(feature = "webp")]
            {
                dispatch::<crate::formats::webp::Codec>(input, opts)
            }
            #[cfg(not(feature = "webp"))]
            {
                Err(ImgOptimError::not_built(ImageFormat::Webp))
            }
        }
    }
}

/// Convert/optimize from in-memory bytes when the input format is known.
pub fn convert_bytes_with_input(
    input: &[u8],
    input_fmt: ImageFormat,
    output_fmt: ImageFormat,
    opts: &OptimizeOptions,
    background: Option<[u8; 3]>,
) -> ResultError<Vec<u8>> {
    if !formats::is_built(output_fmt) {
        return Err(ImgOptimError::not_built(output_fmt));
    }

    opts.validate(output_fmt)?;

    if input_fmt == output_fmt {
        return convert_bytes(input, output_fmt, opts);
    }

    let raw = match input_fmt {
        ImageFormat::Jpeg => {
            #[cfg(feature = "jpeg")]
            {
                crate::formats::jpeg::decode_to_raw(input)?
            }
            #[cfg(not(feature = "jpeg"))]
            {
                return Err(ImgOptimError::not_built(ImageFormat::Jpeg));
            }
        }
        ImageFormat::Png => {
            #[cfg(feature = "png")]
            {
                crate::formats::png::decode_to_raw(input)?
            }
            #[cfg(not(feature = "png"))]
            {
                return Err(ImgOptimError::not_built(ImageFormat::Png));
            }
        }
        ImageFormat::Webp => {
            #[cfg(feature = "webp")]
            {
                crate::formats::webp::decode_to_raw(input)?
            }
            #[cfg(not(feature = "webp"))]
            {
                return Err(ImgOptimError::not_built(ImageFormat::Webp));
            }
        }
    };

    match output_fmt {
        ImageFormat::Jpeg => {
            #[cfg(feature = "jpeg")]
            {
                crate::formats::jpeg::encode_from_raw(&raw, opts, background)
            }
            #[cfg(not(feature = "jpeg"))]
            {
                Err(ImgOptimError::not_built(ImageFormat::Jpeg))
            }
        }
        ImageFormat::Png => {
            #[cfg(feature = "png")]
            {
                crate::formats::png::encode_from_raw(&raw, opts)
            }
            #[cfg(not(feature = "png"))]
            {
                Err(ImgOptimError::not_built(ImageFormat::Png))
            }
        }
        ImageFormat::Webp => {
            #[cfg(feature = "webp")]
            {
                crate::formats::webp::encode_from_raw(&raw, opts)
            }
            #[cfg(not(feature = "webp"))]
            {
                Err(ImgOptimError::not_built(ImageFormat::Webp))
            }
        }
    }
}

#[inline]
fn dispatch<C: ImageCodec>(input: &[u8], opts: &OptimizeOptions) -> ResultError<Vec<u8>> {
    C::optimize(input, opts)
}
