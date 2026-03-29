//! JPEG backend (single-file module).
//!
//! Decoder: zune-jpeg (pure Rust)
//! Encoder: jpeg-encoder (pure Rust)
//!
//! Compatible with the trait-based router defined in `crate::formats::convert`.

use crate::error::{ImgOptimError, ResultError};
use crate::formats::convert::{ImageCodec, OptimizeOptions, RawColor, RawImage};
use crate::formats::ImageFormat;

use jpeg_encoder as je;
use jpeg_encoder::SamplingFactor;
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

pub struct Codec;

impl ImageCodec for Codec {
    const FORMAT: ImageFormat = ImageFormat::Jpeg;

    fn optimize(input: &[u8], opts: &OptimizeOptions) -> ResultError<Vec<u8>> {
        validate_jpeg_options(opts)?;

        // 1) Decode JPEG -> raw pixels
        let decoded = decode_jpeg_zune(input)?;

        // 2) Encode raw pixels -> JPEG
        let out = encode_jpeg(&decoded, opts)?;

        // 3) Metadata hooks (optional): no-op by default in this file.
        Ok(out)
    }
}

/// Optional convenience wrapper (if other code still calls `jpeg::optimize(...)`).
#[inline]
pub fn optimize(input: &[u8], opts: &OptimizeOptions) -> ResultError<Vec<u8>> {
    Codec::optimize(input, opts)
}

/* ----------------------------- Validation ----------------------------- */

fn validate_jpeg_options(opts: &OptimizeOptions) -> ResultError<()> {
    if opts.png_level.is_some() || opts.zopfli {
        return Err(ImgOptimError::InvalidArgs(
            "PNG options are not valid for JPEG".into(),
        ));
    }
    if opts.webp_lossless || opts.webp_method.is_some() {
        return Err(ImgOptimError::InvalidArgs(
            "WebP options are not valid for JPEG".into(),
        ));
    }
    Ok(())
}

/* ----------------------------- Decode (zune-jpeg) ----------------------------- */

#[derive(Debug, Clone)]
struct DecodedJpeg {
    width: u16,
    height: u16,
    pixels: Vec<u8>,
    color: JpegColor,
}

#[derive(Debug, Clone, Copy)]
enum JpegColor {
    L8,
    Rgb8,
}

pub(crate) fn decode_to_raw(input: &[u8]) -> ResultError<RawImage> {
    let decoded = decode_jpeg_zune(input)?;
    let color = match decoded.color {
        JpegColor::L8 => RawColor::L8,
        JpegColor::Rgb8 => RawColor::Rgb8,
    };
    Ok(RawImage {
        width: decoded.width as u32,
        height: decoded.height as u32,
        color,
        pixels: decoded.pixels,
    })
}

pub(crate) fn encode_from_raw(
    raw: &RawImage,
    opts: &OptimizeOptions,
    background: Option<[u8; 3]>,
) -> ResultError<Vec<u8>> {
    if raw.width > u16::MAX as u32 || raw.height > u16::MAX as u32 {
        return Err(ImgOptimError::Processing(
            "JPEG encode: dimensions exceed u16 limits".into(),
        ));
    }

    let (pixels, color) = match raw.color {
        RawColor::L8 => (raw.pixels.clone(), JpegColor::L8),
        RawColor::Rgb8 => (raw.pixels.clone(), JpegColor::Rgb8),
        RawColor::Rgba8 => {
            let bg = background.unwrap_or([255, 255, 255]);
            let rgb = rgba_to_rgb_with_bg(&raw.pixels, bg);
            (rgb, JpegColor::Rgb8)
        }
    };

    let img = DecodedJpeg {
        width: raw.width as u16,
        height: raw.height as u16,
        pixels,
        color,
    };

    encode_jpeg(&img, opts)
}

fn rgba_to_rgb_with_bg(rgba: &[u8], bg: [u8; 3]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len() / 4 * 3);
    for px in rgba.chunks_exact(4) {
        let a = px[3] as u32;
        let inv = 255u32 - a;
        let r = (px[0] as u32 * a + bg[0] as u32 * inv) / 255;
        let g = (px[1] as u32 * a + bg[1] as u32 * inv) / 255;
        let b = (px[2] as u32 * a + bg[2] as u32 * inv) / 255;
        out.push(r as u8);
        out.push(g as u8);
        out.push(b as u8);
    }
    out
}

fn decode_jpeg_zune(input: &[u8]) -> ResultError<DecodedJpeg> {
    let options = DecoderOptions::default();

    // Depending on zune-core minor version, this setter name can differ.
    // If this line fails, replace with the equivalent setter for your version.
    let _ = options.jpeg_set_out_colorspace(ColorSpace::RGB);

    // Depending on zune-jpeg minor version, constructor may differ.
    // If it fails, adapt ONLY this initialization block.
    let cursor = ZCursor::new(input);
    let mut decoder = JpegDecoder::new_with_options(cursor, options);

    let pixels = decoder
        .decode()
        .map_err(|e| ImgOptimError::Processing(format!("JPEG decode failed: {e}")))?;

    // Depending on API, you may have `decoder.info()` or `decoder.dimensions()`.
    let info = decoder
        .info()
        .ok_or_else(|| ImgOptimError::Processing("JPEG decode: missing info".into()))?;

    let width = info.width;
    let height = info.height;

    // With output colorspace forced to RGB, the decoder returns RGB data for non-grayscale inputs.
    let components = info.components as usize;
    let color: JpegColor = match components {
        1 => JpegColor::L8,
        _ => JpegColor::Rgb8,
    };

    Ok(DecodedJpeg {
        width,
        height,
        pixels,
        color,
    })
}

/* ----------------------------- Encode (jpeg-encoder) ----------------------------- */

fn encode_jpeg(img: &DecodedJpeg, opts: &OptimizeOptions) -> ResultError<Vec<u8>> {
    let q = opts
        .quality
        .or(opts.max_quality)
        .unwrap_or(75)
        .clamp(1, 100);

    let mut out = Vec::with_capacity(img.pixels.len() / 2);
    let mut enc = je::Encoder::new(&mut out, q);
    if opts.progressive {
        enc.set_progressive(true);
    }
    if let Some(sampling) = opts.jpeg_sampling {
        enc.set_sampling_factor(map_sampling_factor(sampling));
    }

    let color = match img.color {
        JpegColor::L8 => je::ColorType::Luma,
        JpegColor::Rgb8 => je::ColorType::Rgb,
    };

    enc.encode(&img.pixels, img.width, img.height, color)
        .map_err(|e| ImgOptimError::Processing(format!("JPEG encode failed: {e}")))?;

    Ok(out)
}

fn map_sampling_factor(sampling: crate::formats::convert::JpegSampling) -> SamplingFactor {
    match sampling {
        crate::formats::convert::JpegSampling::S444 => SamplingFactor::R_4_4_4,
        crate::formats::convert::JpegSampling::S422 => SamplingFactor::R_4_2_2,
        crate::formats::convert::JpegSampling::S420 => SamplingFactor::R_4_2_0,
    }
}
