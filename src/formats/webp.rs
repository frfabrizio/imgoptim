//! WebP backend (single-file module).
//!
//! Compatibility:
//! - Implements `ImageCodec` used by the trait-based router in `convert.rs`.
//!
//! Current policy (per your prior requirement):
//! - WebP supports LOSSLESS encoding only.
//! - If `quality` or `webp_method` is provided, we return an explicit error.
//!
//! Implementation:
//! - Uses `image` crate WebP decoder/encoder. Be aware that depending on your
//!   feature set and platform, WebP support may rely on external codecs.
//!   Validate your "100% Rust" requirement at Cargo level (features/deps).

use crate::formats::convert::{ImageCodec, OptimizeOptions, RawColor, RawImage};
use crate::error::{ImgOptimError, ResultError};
use crate::formats::ImageFormat;

use std::io::Cursor;

use image::codecs::webp::{WebPDecoder, WebPEncoder};
use image::{ColorType, ImageDecoder};

pub struct Codec;

impl ImageCodec for Codec {
    const FORMAT: ImageFormat = ImageFormat::Webp;

    fn optimize(input: &[u8], opts: &OptimizeOptions) -> ResultError<Vec<u8>> {
        validate_webp_options(opts)?;

        // 1) Decode WebP -> RGBA8 (safe default)
        let decoded = decode_webp_to_rgba8(input)?;

        // 2) Encode -> WebP lossless
        let mut out = encode_webp_lossless(&decoded)?;

        // 3) Optional metadata preservation hooks (no-op by default)
        if let Some(meta) = read_webp_metadata(input)? {
            write_webp_metadata(&mut out, &meta)?;
        }

        Ok(out)
    }
}

/// Optional convenience wrapper if other modules still call `webp::optimize(...)`.
#[inline]
pub fn optimize(input: &[u8], opts: &OptimizeOptions) -> ResultError<Vec<u8>> {
    Codec::optimize(input, opts)
}

pub(crate) fn decode_to_raw(input: &[u8]) -> ResultError<RawImage> {
    let decoded = decode_webp_to_rgba8(input)?;
    Ok(RawImage {
        width: decoded.width,
        height: decoded.height,
        color: RawColor::Rgba8,
        pixels: decoded.pixels,
    })
}

pub(crate) fn encode_from_raw(raw: &RawImage, _opts: &OptimizeOptions) -> ResultError<Vec<u8>> {
    let rgba = match raw.color {
        RawColor::Rgba8 => raw.pixels.clone(),
        RawColor::Rgb8 => rgb_to_rgba(&raw.pixels),
        RawColor::L8 => luma_to_rgba(&raw.pixels),
    };
    let img = RgbaImage {
        width: raw.width,
        height: raw.height,
        pixels: rgba,
    };
    encode_webp_lossless(&img)
}

/* ----------------------------- Validation ----------------------------- */

fn validate_webp_options(opts: &OptimizeOptions) -> ResultError<()> {
    // Reject non-WebP options early (convert.rs already validates, but redundancy is fine).
    if opts.progressive {
        return Err(ImgOptimError::InvalidOption(
            "progressive is not applicable to WebP".into(),
        ));
    }
    if opts.png_level.is_some() || opts.zopfli {
        return Err(ImgOptimError::InvalidOption(
            "PNG options are not valid for WebP".into(),
        ));
    }

    // Enforce your policy: lossless only.
    if !opts.webp_lossless {
        return Err(ImgOptimError::InvalidOption(
            "WebP: only lossless mode is supported (use --webp-lossless)".into(),
        ));
    }
    if opts.quality.is_some() || opts.max_quality.is_some() {
        return Err(ImgOptimError::InvalidOption(
            "WebP: quality/max_quality are not supported (lossless only)".into(),
        ));
    }
    if opts.webp_method.is_some() {
        return Err(ImgOptimError::InvalidOption(
            "WebP: method is not supported (lossless only)".into(),
        ));
    }

    Ok(())
}

/* ----------------------------- Decode/Encode ----------------------------- */

#[derive(Debug, Clone)]
struct RgbaImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>, // RGBA8
}

/// Decode WebP bytes into RGBA8.
fn decode_webp_to_rgba8(input: &[u8]) -> ResultError<RgbaImage> {
    let cursor = Cursor::new(input);
    let decoder = WebPDecoder::new(cursor)
        .map_err(|e| ImgOptimError::Processing(format!("WebP decoder init failed: {e}")))?;

    let (width, height) = decoder.dimensions();
    let color = decoder.color_type();

    // Decode to raw buffer in the decoder's native output, then normalize.
    let mut raw = vec![0u8; decoder.total_bytes() as usize];
    let decoder = decoder;
    decoder
        .read_image(&mut raw)
        .map_err(|e| ImgOptimError::Processing(format!("WebP decode failed: {e}")))?;

    // Normalize to RGBA8
    let rgba = match color {
        ColorType::Rgba8 => raw,
        ColorType::Rgb8 => rgb_to_rgba(&raw),
        ColorType::L8 => luma_to_rgba(&raw),
        ColorType::La8 => luma_alpha_to_rgba(&raw),

        other => {
            return Err(ImgOptimError::Processing(format!(
                "WebP color type not supported by this backend: {other:?}"
            )));
        }
    };

    // Sanity check
    let expected = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    if rgba.len() != expected {
        return Err(ImgOptimError::Processing(format!(
            "WebP decode produced unexpected buffer size: got {}, expected {} ({}x{} RGBA)",
            rgba.len(),
            expected,
            width,
            height
        )));
    }

    Ok(RgbaImage {
        width,
        height,
        pixels: rgba,
    })
}

fn encode_webp_lossless(img: &RgbaImage) -> ResultError<Vec<u8>> {
    let mut out = Vec::<u8>::new();

    // The `image` crate exposes a lossless WebP encoder constructor.
    let enc = WebPEncoder::new_lossless(&mut out);

    enc.encode(
        &img.pixels,
        img.width,
        img.height,
        ColorType::Rgba8.into(),
    )
    .map_err(|e| ImgOptimError::Processing(format!("WebP encode failed: {e}")))?;

    Ok(out)
}

/* ----------------------------- Pixel helpers ----------------------------- */

fn rgb_to_rgba(rgb: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgb.len() / 3 * 4);
    for px in rgb.chunks_exact(3) {
        out.extend_from_slice(&[px[0], px[1], px[2], 255]);
    }
    out
}

fn luma_to_rgba(l: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(l.len() * 4);
    for &v in l {
        out.extend_from_slice(&[v, v, v, 255]);
    }
    out
}

fn luma_alpha_to_rgba(la: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(la.len() / 2 * 4);
    for px in la.chunks_exact(2) {
        let v = px[0];
        let a = px[1];
        out.extend_from_slice(&[v, v, v, a]);
    }
    out
}

/* ----------------------------- Metadata hooks ----------------------------- */

/// Minimal metadata container (adapt to your project needs).
#[derive(Debug, Clone, Default)]
pub struct WebpMetadata {
    pub exif: Option<Vec<u8>>,
    pub xmp: Option<Vec<u8>>,
    pub icc: Option<Vec<u8>>,
}

/// Read metadata from an existing WebP.
///
/// If you already have a RIFF/WEBP chunk parser in your old webp.rs,
/// plug it here and return `Some(...)`.
fn read_webp_metadata(_input: &[u8]) -> ResultError<Option<WebpMetadata>> {
    Ok(None)
}

/// Write metadata back into the output WebP buffer.
///
/// Implement this if you want metadata preservation.
/// Default: no-op.
fn write_webp_metadata(_output_webp: &mut Vec<u8>, _meta: &WebpMetadata) -> ResultError<()> {
    Ok(())
}
