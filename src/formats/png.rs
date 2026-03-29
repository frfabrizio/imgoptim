//! PNG backend (single-file module).
//!
//! Decoder/Encoder: `png` crate (pure Rust)
//!
//! This module exposes `Codec` implementing `ImageCodec` so it works with the
//! trait-based router in `convert.rs`.
//!
//! Supported options from `OptimizeOptions`:
//! - `png_level` (0..9) -> mapped to `png::Compression`
//! - `zopfli` -> runs an oxipng zopfli pass after encoding
//!
//! Notes:
//! - This implementation round-trips pixels (decode -> encode). It is safe but may not keep
//!   the original PNG structure (chunks ordering, ancillary chunks) unless you re-inject metadata.
//! - Metadata preservation hooks are provided as no-op by default.

use crate::error::{ImgOptimError, ResultError};
use crate::formats::convert::{ImageCodec, OptimizeOptions, RawColor, RawImage};
use crate::formats::ImageFormat;

use png::{BitDepth, ColorType, Decoder, Encoder, Transformations};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use std::{io::Write, thread};

pub struct Codec;

impl ImageCodec for Codec {
    const FORMAT: ImageFormat = ImageFormat::Png;

    fn optimize(input: &[u8], opts: &OptimizeOptions) -> ResultError<Vec<u8>> {
        validate_png_options(opts)?;

        // 1) Decode PNG into a normalized buffer (RGB/RGBA, 8-bit)
        let decoded = decode_png_normalized(input)?;

        // 2) Encode back with chosen compression level (+ optional zopfli pass)
        let mut out = encode_png_normalized(&decoded, opts)?;

        // 3) Optional metadata preservation hooks (no-op by default)
        if let Some(meta) = read_png_metadata(input)? {
            write_png_metadata(&mut out, &meta)?;
        }

        Ok(out)
    }
}

/// Optional convenience wrapper if other modules still call `png::optimize(...)`.
#[inline]
pub fn optimize(input: &[u8], opts: &OptimizeOptions) -> ResultError<Vec<u8>> {
    Codec::optimize(input, opts)
}

/* ----------------------------- Validation ----------------------------- */

fn validate_png_options(opts: &OptimizeOptions) -> ResultError<()> {
    // Reject non-PNG options early (convert.rs already validates, but redundancy is fine).
    if opts.quality.is_some() || opts.max_quality.is_some() || opts.progressive {
        return Err(ImgOptimError::InvalidOption(
            "JPEG quality/max_quality/progressive are not applicable to PNG".into(),
        ));
    }
    if opts.webp_lossless || opts.webp_method.is_some() {
        return Err(ImgOptimError::InvalidOption(
            "WebP options are not valid for PNG".into(),
        ));
    }

    // Validate png_level range if provided
    if let Some(lvl) = opts.png_level {
        if lvl > 9 {
            return Err(ImgOptimError::InvalidOption(
                "png_level must be in range 0..9".into(),
            ));
        }
    }

    Ok(())
}

/* ----------------------------- Decode/Encode ----------------------------- */

#[derive(Debug, Clone)]
struct PngImage {
    width: u32,
    height: u32,
    color: NormalizedColor,
    pixels: Vec<u8>, // RGB8 or RGBA8
}

#[derive(Debug, Clone, Copy)]
enum NormalizedColor {
    Rgb8,
    Rgba8,
}

pub(crate) fn decode_to_raw(input: &[u8]) -> ResultError<RawImage> {
    let img = decode_png_normalized(input)?;
    let color = match img.color {
        NormalizedColor::Rgb8 => RawColor::Rgb8,
        NormalizedColor::Rgba8 => RawColor::Rgba8,
    };
    Ok(RawImage {
        width: img.width,
        height: img.height,
        color,
        pixels: img.pixels,
    })
}

pub(crate) fn encode_from_raw(raw: &RawImage, opts: &OptimizeOptions) -> ResultError<Vec<u8>> {
    let (color, pixels) = match raw.color {
        RawColor::L8 => (NormalizedColor::Rgb8, l8_to_rgb(&raw.pixels)),
        RawColor::Rgb8 => (NormalizedColor::Rgb8, raw.pixels.clone()),
        RawColor::Rgba8 => (NormalizedColor::Rgba8, raw.pixels.clone()),
    };
    let img = PngImage {
        width: raw.width,
        height: raw.height,
        color,
        pixels,
    };
    encode_png_normalized(&img, opts)
}

fn l8_to_rgb(l: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(l.len() * 3);
    for &v in l {
        out.extend_from_slice(&[v, v, v]);
    }
    out
}

/// Decode PNG and normalize into RGB8/RGBA8, 8-bit, expanded.
fn decode_png_normalized(input: &[u8]) -> ResultError<PngImage> {
    let mut decoder = Decoder::new(std::io::Cursor::new(input));

    // EXPAND:
    // - palette -> RGB/RGBA
    // - grayscale -> 8-bit
    // - apply tRNS (transparency) where relevant
    // STRIP_16 -> downcast 16-bit to 8-bit (deterministic)
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);

    let mut reader = decoder
        .read_info()
        .map_err(|e| ImgOptimError::Processing(format!("png read_info failed: {e}")))?;

    let mut buf = vec![0u8; reader.output_buffer_size().expect("REASON")];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| ImgOptimError::Processing(format!("png decode failed: {e}")))?;

    let bytes = &buf[..info.buffer_size()];
    let (w, h) = (info.width, info.height);

    // Normalize to RGB/RGBA
    let (color, pixels) = match info.color_type {
        ColorType::Rgb => (NormalizedColor::Rgb8, bytes.to_vec()),
        ColorType::Rgba => (NormalizedColor::Rgba8, bytes.to_vec()),

        // After EXPAND, these can still appear depending on source;
        // normalize deterministically.
        ColorType::Grayscale => {
            let mut out = Vec::with_capacity((w * h * 3) as usize);
            for &v in bytes {
                out.extend_from_slice(&[v, v, v]);
            }
            (NormalizedColor::Rgb8, out)
        }
        ColorType::GrayscaleAlpha => {
            let mut out = Vec::with_capacity((w * h * 4) as usize);
            for px in bytes.chunks_exact(2) {
                let v = px[0];
                let a = px[1];
                out.extend_from_slice(&[v, v, v, a]);
            }
            (NormalizedColor::Rgba8, out)
        }

        // Palette should usually be expanded by EXPAND, but keep a safe fallback.
        ColorType::Indexed => {
            return Err(ImgOptimError::Processing(
                "png indexed color remained after EXPAND; unsupported by this backend".into(),
            ));
        }
    };

    // Sanity checks
    let expected = match color {
        NormalizedColor::Rgb8 => (w as usize).saturating_mul(h as usize).saturating_mul(3),
        NormalizedColor::Rgba8 => (w as usize).saturating_mul(h as usize).saturating_mul(4),
    };
    if pixels.len() != expected {
        return Err(ImgOptimError::Processing(format!(
            "png decode produced unexpected buffer size: got {}, expected {}",
            pixels.len(),
            expected
        )));
    }

    Ok(PngImage {
        width: w,
        height: h,
        color,
        pixels,
    })
}

fn encode_png_normalized(img: &PngImage, opts: &OptimizeOptions) -> ResultError<Vec<u8>> {
    let mut out = Vec::<u8>::new();
    let mut enc = Encoder::new(&mut out, img.width, img.height);

    // Color + depth
    match img.color {
        NormalizedColor::Rgb8 => enc.set_color(ColorType::Rgb),
        NormalizedColor::Rgba8 => enc.set_color(ColorType::Rgba),
    }
    enc.set_depth(BitDepth::Eight);

    // Compression mapping:
    // png crate doesn't expose "level 0..9" directly in a stable way.
    // We map it to semantic compression modes.
    let compression = map_png_level_to_compression(opts.png_level);
    enc.set_compression(compression);

    // You can also tune filter strategy if you want a deterministic baseline:
    // enc.set_filter(png::FilterType::Sub);

    {
        let mut writer = enc
            .write_header()
            .map_err(|e| ImgOptimError::Processing(format!("png write_header failed: {e}")))?;

        writer
            .write_image_data(&img.pixels)
            .map_err(|e| ImgOptimError::Processing(format!("png write_image_data failed: {e}")))?;
    }

    apply_zopfli_if_needed(out, opts)
}

fn apply_zopfli_if_needed(bytes: Vec<u8>, opts: &OptimizeOptions) -> ResultError<Vec<u8>> {
    if !opts.zopfli
        && opts.zopfli_iteration_count.is_none()
        && opts.zopfli_max_block_splits.is_none()
        && opts.zopfli_timeout_secs.is_none()
    {
        return Ok(bytes);
    }

    let show_progress = opts.zopfli_progress;
    let mut oxi_opts = if let Some(level) = opts.png_level {
        oxipng::Options::from_preset(level.min(6))
    } else {
        oxipng::Options::default()
    };
    oxi_opts.strip = oxipng::StripChunks::None;
    let mut zopts = oxipng::ZopfliOptions::default();
    if let Some(iter) = opts.zopfli_iteration_count {
        zopts.iteration_count = std::num::NonZeroU64::new(iter).ok_or_else(|| {
            ImgOptimError::InvalidArgs("zopfli iteration_count must be >= 1".into())
        })?;
    }
    if let Some(splits) = opts.zopfli_max_block_splits {
        zopts.maximum_block_splits = splits;
    }
    oxi_opts.deflater = oxipng::Deflater::Zopfli(zopts);
    if let Some(secs) = opts.zopfli_timeout_secs {
        oxi_opts.timeout = Some(Duration::from_secs(secs));
    }

    let start = if show_progress {
        Some(Instant::now())
    } else {
        None
    };

    let stop = Arc::new(AtomicBool::new(false));
    let timeout_handle = if show_progress {
        if let Some(secs) = opts.zopfli_timeout_secs {
            let stop_flag = Arc::clone(&stop);
            Some(thread::spawn(move || {
                for remaining in (1..=secs).rev() {
                    if stop_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    print!("\rZopfli timeout: {remaining}s");
                    let _ = std::io::stdout().flush();
                    thread::sleep(Duration::from_secs(1));
                }
            }))
        } else {
            None
        }
    } else {
        None
    };

    if show_progress {
        println!("Zopfli: start");
    }

    let result = oxipng::optimize_from_memory(&bytes, &oxi_opts)
        .map_err(|e| ImgOptimError::Processing(format!("oxipng failed: {e}")));

    stop.store(true, Ordering::Relaxed);
    if let Some(handle) = timeout_handle {
        let _ = handle.join();
        println!();
    }

    if let Some(start) = start {
        let elapsed = start.elapsed().as_secs_f32();
        if result.is_ok() {
            println!("Zopfli: done in {elapsed:.1}s");
        } else {
            println!("Zopfli: failed after {elapsed:.1}s");
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_png_bytes() -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, 1, 1);
            enc.set_color(ColorType::Rgb);
            enc.set_depth(BitDepth::Eight);
            let mut writer = enc.write_header().expect("png write_header");
            writer
                .write_image_data(&[0u8, 0u8, 0u8])
                .expect("png write_image_data");
        }
        out
    }

    #[test]
    fn zopfli_optimize_from_memory_works() {
        let bytes = tiny_png_bytes();
        let opts = OptimizeOptions {
            quality: None,
            max_quality: None,
            progressive: false,
            jpeg_sampling: None,
            png_level: Some(6),
            zopfli: true,
            zopfli_iteration_count: Some(1),
            zopfli_max_block_splits: Some(1),
            zopfli_timeout_secs: Some(1),
            zopfli_progress: false,
            webp_lossless: false,
            webp_method: None,
        };

        let out = apply_zopfli_if_needed(bytes, &opts).expect("zopfli optimize");
        assert!(out.starts_with(b"\x89PNG\r\n\x1a\n"), "output is not PNG");
        assert!(!out.is_empty(), "output should not be empty");
    }
}

fn map_png_level_to_compression(level: Option<u8>) -> png::Compression {
    match level.unwrap_or(6) {
        // 0: no compression
        0 => png::Compression::NoCompression,
        // 1..2: prioritize speed
        1..=2 => png::Compression::Fastest,
        // 3..6: default tradeoff
        3..=6 => png::Compression::Balanced,
        // 7..9: best compression
        _ => png::Compression::High,
    }
}

/* ----------------------------- Metadata hooks ----------------------------- */

pub fn inject_png_meta(
    input: &[u8],
    exif: Option<&[u8]>,
    icc: Option<&[u8]>,
    xmp: Option<&[u8]>,
) -> ResultError<Vec<u8>> {
    const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    const PNG_XMP_KEYWORD: &[u8] = b"XML:com.adobe.xmp";

    if input.len() < 8 || &input[..8] != PNG_SIG {
        return Err(ImgOptimError::Processing("invalid PNG signature".into()));
    }

    let extra =
        exif.map_or(0, |v| v.len()) + icc.map_or(0, |v| v.len()) + xmp.map_or(0, |v| v.len());
    let mut out = Vec::with_capacity(input.len() + extra + 64);
    out.extend_from_slice(&input[..8]);

    let mut pos = 8usize;
    while pos + 12 <= input.len() {
        let len = u32::from_be_bytes(input[pos..pos + 4].try_into().unwrap()) as usize;
        let ctype: [u8; 4] = input[pos + 4..pos + 8].try_into().unwrap();
        let chunk_total = 12 + len;

        if pos + chunk_total > input.len() {
            return Err(ImgOptimError::Processing("corrupt PNG chunk size".into()));
        }

        let data = &input[pos + 8..pos + 8 + len];

        if &ctype == b"IEND" {
            if let Some(exif) = exif {
                out.extend_from_slice(&make_png_chunk(*b"eXIf", exif));
            }
            if let Some(icc) = icc {
                out.extend_from_slice(&make_png_chunk(*b"iCCP", icc));
            }
            if let Some(xmp) = xmp {
                out.extend_from_slice(&build_png_itxt_xmp(PNG_XMP_KEYWORD, xmp));
            }

            out.extend_from_slice(&input[pos..pos + chunk_total]);
            return Ok(out);
        }

        if !should_skip_chunk(&ctype, data, PNG_XMP_KEYWORD) {
            out.extend_from_slice(&input[pos..pos + chunk_total]);
        }

        pos += chunk_total;
    }

    Err(ImgOptimError::Processing("PNG missing IEND chunk".into()))
}

fn should_skip_chunk(ctype: &[u8; 4], data: &[u8], xmp_keyword: &[u8]) -> bool {
    if ctype == b"eXIf" || ctype == b"iCCP" {
        return true;
    }
    if ctype == b"iTXt" {
        return itxt_keyword_is_xmp(data, xmp_keyword);
    }
    false
}

fn build_png_itxt_xmp(xmp_keyword: &[u8], xmp: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(xmp_keyword);
    data.push(0);
    data.push(0);
    data.push(0);
    data.push(0);
    data.push(0);
    data.extend_from_slice(xmp);

    make_png_chunk(*b"iTXt", &data)
}

fn itxt_keyword_is_xmp(data: &[u8], xmp_keyword: &[u8]) -> bool {
    let nul = match data.iter().position(|&b| b == 0) {
        Some(p) => p,
        None => return false,
    };
    &data[..nul] == xmp_keyword
}

fn make_png_chunk(ctype: [u8; 4], data: &[u8]) -> Vec<u8> {
    use crc32fast::Hasher;

    let mut out = Vec::with_capacity(12 + data.len());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&ctype);
    out.extend_from_slice(data);

    let mut h = Hasher::new();
    h.update(&ctype);
    h.update(data);
    out.extend_from_slice(&h.finalize().to_be_bytes());

    out
}

/// Minimal metadata container (adapt to your project needs).
#[derive(Debug, Clone, Default)]
pub struct PngMetadata {
    pub exif: Option<Vec<u8>>,
    pub xmp: Option<Vec<u8>>,
    pub icc: Option<Vec<u8>>,
    // Optionally: textual chunks, etc.
}

/// Read metadata from an existing PNG.
///
/// If you already have a metadata pipeline (e.g., reading eXIf, iCCP, iTXt),
/// plug it here and return `Some(...)`.
pub(crate) fn read_png_metadata(input: &[u8]) -> ResultError<Option<PngMetadata>> {
    const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

    if input.len() < 8 || &input[..8] != PNG_SIG {
        return Err(ImgOptimError::Processing("invalid PNG signature".into()));
    }

    let mut meta = PngMetadata::default();
    let mut pos = 8usize;

    while pos + 12 <= input.len() {
        let len = u32::from_be_bytes(input[pos..pos + 4].try_into().unwrap()) as usize;
        let ctype: [u8; 4] = input[pos + 4..pos + 8].try_into().unwrap();
        let chunk_total = 12 + len;

        if pos + chunk_total > input.len() {
            return Err(ImgOptimError::Processing("corrupt PNG chunk size".into()));
        }

        let data = &input[pos + 8..pos + 8 + len];

        if &ctype == b"eXIf" {
            meta.exif = Some(data.to_vec());
        } else if &ctype == b"iCCP" {
            meta.icc = Some(data.to_vec());
        } else if &ctype == b"iTXt" {
            if let Some(xmp) = parse_png_itxt_xmp(data) {
                meta.xmp = Some(xmp);
            }
        } else if &ctype == b"IEND" {
            break;
        }

        pos += chunk_total;
    }

    if meta.exif.is_none() && meta.icc.is_none() && meta.xmp.is_none() {
        Ok(None)
    } else {
        Ok(Some(meta))
    }
}

/// Write metadata back into the output PNG buffer.
///
/// Implement this if you want metadata preservation.
/// Default: no-op.
fn write_png_metadata(_output_png: &mut Vec<u8>, _meta: &PngMetadata) -> ResultError<()> {
    Ok(())
}

fn parse_png_itxt_xmp(data: &[u8]) -> Option<Vec<u8>> {
    let nul = data.iter().position(|&b| b == 0)?;
    let keyword = &data[..nul];
    if keyword != b"XML:com.adobe.xmp" {
        return None;
    }
    let mut p = nul + 1;

    let compression_flag = *data.get(p)?;
    let _compression_method = *data.get(p + 1)?;
    p += 2;

    let nul2 = data.get(p..)?.iter().position(|&b| b == 0)?;
    p += nul2 + 1;

    let nul3 = data.get(p..)?.iter().position(|&b| b == 0)?;
    p += nul3 + 1;

    let text = data.get(p..)?;
    if compression_flag == 0 {
        Some(text.to_vec())
    } else {
        None
    }
}
