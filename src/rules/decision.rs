use std::path::Path;
use std::{io, io::Write};

use crate::cli::{Fmt, Mode, Opts, TargetSize};
use crate::error::ImgOptimError;
use crate::formats::convert::{RawColor, RawImage};
use crate::formats::resize::{resize_rgb_bilinear, resize_rgba_bilinear};
use crate::formats::{self, ImageFormat};
use crate::rules::naming::make_output_path;
use crate::rules::resize::parse_resize_spec;
use crate::rules::threshold::{gain_percent, should_replace};
use std::sync::Mutex;

use once_cell::sync::Lazy;

#[derive(Default)]
struct Totals {
    files: u64,
    optimized: u64,
    converted: u64,
    kept: u64,
    errors: u64,
    bytes_in: u64,
    bytes_out: u64,
}

static TOTALS: Lazy<Mutex<Totals>> = Lazy::new(|| Mutex::new(Totals::default()));

#[derive(Clone, Copy)]
struct ImageInfo {
    width: u32,
    height: u32,
    bit_depth: u8,
    has_exif: bool,
}

#[derive(Clone, Copy)]
struct SummaryStats {
    mode: Mode,
    input_fmt: ImageFormat,
    output_fmt: ImageFormat,
    old_bytes: u64,
    new_bytes: u64,
    gain: f32,
}

/// Map internal ImageFormat -> CLI enum (for --only/--skip filtering)
fn fmt_to_cli(fmt: ImageFormat) -> Fmt {
    match fmt {
        ImageFormat::Jpeg => Fmt::Jpeg,
        ImageFormat::Png => Fmt::Png,
        ImageFormat::Webp => Fmt::Webp,
    }
}

/// Apply --only / --skip filtering after detection.
fn passes_only_skip(fmt: ImageFormat, opts: &Opts) -> bool {
    let f = fmt_to_cli(fmt);
    if !opts.only.is_empty() && !opts.only.contains(&f) {
        return false;
    }
    if opts.skip.contains(&f) {
        return false;
    }
    true
}

/// For threshold comparison, use existing output size if present, otherwise input size.
fn old_size_for_target(input: &Path, out: &Path, overwrite: bool) -> Result<u64, ImgOptimError> {
    if out.exists() && !overwrite {
        Ok(std::fs::metadata(out)?.len())
    } else {
        Ok(std::fs::metadata(input)?.len())
    }
}

fn bit_depth_from_color(color: RawColor) -> u8 {
    match color {
        RawColor::L8 => 8,
        RawColor::Rgb8 => 24,
        RawColor::Rgba8 => 32,
    }
}

fn image_info_from_raw(raw: &RawImage, input_fmt: ImageFormat, input_bytes: &[u8]) -> ImageInfo {
    ImageInfo {
        width: raw.width,
        height: raw.height,
        bit_depth: bit_depth_from_color(raw.color),
        has_exif: crate::formats::metadata::has_exif(input_fmt, input_bytes),
    }
}

fn format_jpegoptim_summary(input: &Path, info: ImageInfo, stats: SummaryStats) -> String {
    let name = input
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_else(|| input.display().to_string().into());
    let mut details = format!(
        "{name} {}x{} {}bit",
        info.width, info.height, info.bit_depth
    );
    if stats.input_fmt == ImageFormat::Jpeg {
        details.push_str(" N");
    }
    if info.has_exif {
        details.push_str(" Exif");
    }

    let action = if stats.mode == Mode::Optimize && stats.input_fmt == stats.output_fmt {
        "optimized."
    } else {
        "converted."
    };

    format!(
        "{details} [OK] {} --> {} bytes ({:.2}%), {action}",
        stats.old_bytes, stats.new_bytes, stats.gain
    )
}

fn format_zopfli_status(opts: &Opts) -> String {
    let iteration = opts
        .zopfli_iteration_count
        .map(|v| v.to_string())
        .unwrap_or_else(|| "default".to_string());
    let splits = opts
        .zopfli_max_block_splits
        .map(|v| v.to_string())
        .unwrap_or_else(|| "default".to_string());
    let timeout = opts
        .zopfli_timeout_secs
        .map(|v| format!("{v}s"))
        .unwrap_or_else(|| "none".to_string());
    format!("Zopfli: iteration_count={iteration} max_block_splits={splits} timeout={timeout}")
}

fn zopfli_requested(opts: &Opts) -> bool {
    opts.zopfli
        || opts.zopfli_iteration_count.is_some()
        || opts.zopfli_max_block_splits.is_some()
        || opts.zopfli_timeout_secs.is_some()
}

/// Build the codec-layer options expected by `formats::convert` (trait-based router).
///
/// Note: current CLI (`cli.rs`) does not expose `png_level`, `zopfli`, `webp_method`,
/// so we provide safe defaults here.
fn build_optimize_options(
    opts: &Opts,
    out_fmt: ImageFormat,
) -> crate::formats::convert::OptimizeOptions {
    // Progressive: CLI has all_normal/all_progressive. We model progressive as a single bool.
    let progressive = opts.all_progressive;

    // WebP lossless policy: in Convert mode, we can use `convert.lossless` when output is WebP.
    let webp_lossless = if opts.webp_lossless {
        true
    } else {
        match (opts.mode, out_fmt) {
            (Mode::Convert, ImageFormat::Webp) => {
                opts.convert.as_ref().map(|c| !c.lossy).unwrap_or(true)
            }
            (Mode::Optimize, ImageFormat::Webp) => true,
            _ => false,
        }
    };

    crate::formats::convert::OptimizeOptions {
        quality: opts.quality,
        max_quality: opts.max_quality,
        progressive,

        // PNG
        png_level: opts.png_level,
        zopfli: opts.zopfli,
        zopfli_iteration_count: opts.zopfli_iteration_count,
        zopfli_max_block_splits: opts.zopfli_max_block_splits,
        zopfli_timeout_secs: opts.zopfli_timeout_secs,
        zopfli_progress: !matches!(opts.verbosity, crate::cli::Verbosity::Quiet),

        // WebP (lossless-only policy supported)
        webp_lossless,
        webp_method: opts.webp_method,
    }
}

pub fn process_one(input: &Path, opts: &Opts) -> Result<String, ImgOptimError> {
    // 1) Detect input format
    let detected = formats::detect::detect_format(input)?.ok_or(ImgOptimError::UnknownFormat)?;

    // 2) Policy A: must be built into the binary
    if !formats::is_built(detected) {
        return Err(ImgOptimError::not_built(detected));
    }

    // 3) Apply --only/--skip after detection
    if !passes_only_skip(detected, opts) {
        return Ok(format!(
            "{}: skipped by --only/--skip (detected {})",
            input.display(),
            detected
        ));
    }

    // 4) Convert mode: optional --input filter
    if opts.mode == Mode::Convert {
        if let Some(cv) = &opts.convert {
            if let Some(expected) = cv.input {
                if expected != fmt_to_cli(detected) {
                    return Ok(format!(
                        "{}: skipped by convert --input (detected {})",
                        input.display(),
                        detected
                    ));
                }
            }
        }
    }

    // 5) Compute output path
    let out_path = make_output_path(input, detected, opts)?;

    // 6) Produce candidate output bytes via the new trait-based convert router
    let input_bytes = std::fs::read(input)?;
    let want_details = !matches!(opts.verbosity, crate::cli::Verbosity::Quiet);
    let mut info: Option<ImageInfo> = None;

    // Determine output format
    let out_fmt = match opts.mode {
        Mode::Optimize => detected,
        Mode::Convert => {
            let cv = opts
                .convert
                .as_ref()
                .ok_or_else(|| ImgOptimError::InvalidArgs("convert options missing".into()))?;
            match cv.output {
                Fmt::Jpeg => ImageFormat::Jpeg,
                Fmt::Png => ImageFormat::Png,
                Fmt::Webp => ImageFormat::Webp,
            }
        }
    };

    // Build codec options expected by formats::convert
    let cv_opts = build_optimize_options(opts, out_fmt);

    if zopfli_requested(opts)
        && out_fmt == ImageFormat::Png
        && !matches!(opts.verbosity, crate::cli::Verbosity::Quiet)
    {
        println!("{}", format_zopfli_status(opts));
    }

    let background = if out_fmt == ImageFormat::Jpeg {
        let bg = opts
            .convert
            .as_ref()
            .map(|c| c.background.as_str())
            .unwrap_or("#ffffff");
        let rgb = crate::rules::color::parse_hex_rgb(bg)?;
        Some([rgb.r, rgb.g, rgb.b])
    } else {
        None
    };

    let resize_spec = if opts.mode == Mode::Convert {
        opts.convert
            .as_ref()
            .and_then(|c| c.resize.as_deref())
            .map(parse_resize_spec)
            .transpose()?
    } else {
        None
    };

    if opts.target_size_parsed.is_some() && out_fmt != ImageFormat::Jpeg {
        return Err(ImgOptimError::InvalidArgs(
            "--size is only supported with --output jpeg".into(),
        ));
    }

    let mut new_bytes = if resize_spec.is_some() || opts.target_size_parsed.is_some() {
        let mut raw = decode_raw(&input_bytes, detected)?;
        if want_details {
            info = Some(image_info_from_raw(&raw, detected, &input_bytes));
        }
        if let Some(spec) = resize_spec {
            let fit = opts
                .convert
                .as_ref()
                .map(|c| c.fit)
                .unwrap_or(crate::cli::FitMode::Contain);
            raw = apply_resize(raw, spec, fit);
        }

        if let Some(target) = opts.target_size_parsed {
            let input_size = std::fs::metadata(input)?.len();
            let target_bytes = match target {
                TargetSize::KiloBytes(kb) => kb * 1024,
                TargetSize::Percent(p) => (input_size * (p as u64)) / 100,
            };
            let max_q = opts
                .max_quality
                .or(opts.quality)
                .unwrap_or(100)
                .clamp(1, 100);
            let (bytes, reached) =
                encode_jpeg_to_target_size(&raw, &cv_opts, background, target_bytes, max_q)?;
            if !reached && !opts.force {
                let old_bytes = old_size_for_target(input, &out_path, opts.overwrite)?;
                let summary = format!(
                    "{}: kept (target not reached) old={}B new={}B -> {}",
                    input.display(),
                    old_bytes,
                    bytes.len(),
                    out_path.display()
                );
                record_success(
                    opts.mode,
                    detected,
                    out_fmt,
                    old_bytes,
                    bytes.len() as u64,
                    false,
                );
                return Ok(summary);
            }
            bytes
        } else {
            encode_from_raw(&raw, out_fmt, &cv_opts, background)?
        }
    } else if want_details {
        let raw = decode_raw(&input_bytes, detected)?;
        info = Some(image_info_from_raw(&raw, detected, &input_bytes));
        encode_from_raw(&raw, out_fmt, &cv_opts, background)?
    } else {
        crate::formats::convert::convert_bytes_with_input(
            &input_bytes,
            detected,
            out_fmt,
            &cv_opts,
            background,
        )?
    };

    let strip_any = opts.strip.strip_all
        || opts.strip.strip_exif
        || opts.strip.strip_xmp
        || opts.strip.strip_iptc
        || opts.strip.strip_icc
        || opts.strip.strip_com;

    if opts.strip.keep_metadata || !strip_any {
        new_bytes = crate::formats::metadata::preserve_metadata(
            detected,
            out_fmt,
            &input_bytes,
            &new_bytes,
        )?;
    } else {
        new_bytes = crate::formats::metadata::strip_metadata(out_fmt, &new_bytes, &opts.strip)?;
    }

    let allow_tag = !opts.strip.strip_all || opts.strip.keep_metadata;

    // Apply tag category (if requested) *after* conversion/optimization
    let new_bytes = if allow_tag {
        if let Some(cat) = opts.tag_category.as_deref() {
            crate::formats::metadata::apply_tag_category(out_fmt, &new_bytes, cat)?
        } else {
            new_bytes
        }
    } else {
        new_bytes
    };

    // 7) Threshold decision BEFORE writing
    let old_bytes = old_size_for_target(input, &out_path, opts.overwrite)?;
    let new_len = new_bytes.len() as u64;

    let replace = should_replace(old_bytes, new_len, opts.threshold_percent, opts.force)?;
    let gain = gain_percent(old_bytes, new_len);

    // 8) Dry-run
    if opts.dry_run {
        let summary = format!(
            "{}: {} -> {} (dry-run, old={}B new={}B gain={:.2}% replace={})",
            input.display(),
            detected,
            out_path.display(),
            old_bytes,
            new_len,
            gain,
            replace
        );
        record_success(opts.mode, detected, out_fmt, old_bytes, new_len, replace);
        return Ok(summary);
    }

    // 9) If we should not replace, do nothing
    if !replace {
        let summary = format!(
            "{}: kept (threshold) old={}B new={}B gain={:.2}% -> {}",
            input.display(),
            old_bytes,
            new_len,
            gain,
            out_path.display()
        );
        record_success(opts.mode, detected, out_fmt, old_bytes, new_len, false);
        return Ok(summary);
    }

    // 10) Overwrite policy
    if out_path.exists() && !opts.overwrite {
        return Err(ImgOptimError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("output exists: {}", out_path.display()),
        )));
    }

    // 11) Ensure output directory exists (ask before creating)
    ensure_output_dir(&out_path)?;

    // 12) Atomic write
    let mut w = crate::io::atomic_write::AtomicWriter::new(&out_path)?;
    w.write_all(&new_bytes)?;
    w.commit(opts.overwrite)?;

    // 13) Preserve timestamps
    if opts.preserve {
        crate::io::fsmeta::preserve_timestamps(input, &out_path)?;
    }

    let summary = if let Some(info) = info {
        let stats = SummaryStats {
            mode: opts.mode,
            input_fmt: detected,
            output_fmt: out_fmt,
            old_bytes,
            new_bytes: new_len,
            gain,
        };
        format_jpegoptim_summary(input, info, stats)
    } else {
        format!(
            "{}: {} -> {} (written, gain={:.2}%)",
            input.display(),
            detected,
            out_path.display(),
            gain
        )
    };
    record_success(opts.mode, detected, out_fmt, old_bytes, new_len, true);
    Ok(summary)
}

fn ensure_output_dir(out_path: &Path) -> Result<(), ImgOptimError> {
    let dir = out_path
        .parent()
        .ok_or_else(|| ImgOptimError::InvalidArgs("output has no parent directory".into()))?;

    if dir.exists() {
        return Ok(());
    }

    let mut stderr = io::stderr();
    write!(
        stderr,
        "Destination directory does not exist: {}. Create it? [y/N]: ",
        dir.display()
    )?;
    stderr.flush()?;

    let mut input = String::new();
    let bytes = io::stdin().read_line(&mut input)?;
    if bytes == 0 {
        return Err(ImgOptimError::InvalidArgs(
            "destination directory does not exist; no input available".into(),
        ));
    }

    let answer = input.trim().to_ascii_lowercase();
    let yes = matches!(answer.as_str(), "y" | "yes" | "o" | "oui");
    if yes {
        std::fs::create_dir_all(dir)?;
        Ok(())
    } else {
        Err(ImgOptimError::InvalidArgs(
            "destination directory does not exist".into(),
        ))
    }
}

fn record_success(
    mode: Mode,
    input_fmt: ImageFormat,
    output_fmt: ImageFormat,
    old_bytes: u64,
    new_bytes: u64,
    replaced: bool,
) {
    let mut totals = TOTALS.lock().expect("totals mutex");
    totals.files += 1;
    totals.bytes_in += old_bytes;
    if replaced {
        totals.bytes_out += new_bytes;
        if output_fmt == input_fmt && mode == Mode::Optimize {
            totals.optimized += 1;
        } else {
            totals.converted += 1;
        }
    } else {
        totals.kept += 1;
    }
}

pub fn record_error() {
    let mut totals = TOTALS.lock().expect("totals mutex");
    totals.errors += 1;
}

pub fn print_totals() {
    let totals = TOTALS.lock().expect("totals mutex");
    let saved = totals.bytes_in.saturating_sub(totals.bytes_out);
    println!(
        "totals: files={} optimized={} converted={} kept={} errors={} saved={}B",
        totals.files, totals.optimized, totals.converted, totals.kept, totals.errors, saved
    );
}

fn decode_raw(input: &[u8], fmt: ImageFormat) -> Result<RawImage, ImgOptimError> {
    match fmt {
        ImageFormat::Jpeg => {
            #[cfg(feature = "jpeg")]
            {
                crate::formats::jpeg::decode_to_raw(input)
            }
            #[cfg(not(feature = "jpeg"))]
            {
                Err(ImgOptimError::not_built(ImageFormat::Jpeg))
            }
        }
        ImageFormat::Png => {
            #[cfg(feature = "png")]
            {
                crate::formats::png::decode_to_raw(input)
            }
            #[cfg(not(feature = "png"))]
            {
                Err(ImgOptimError::not_built(ImageFormat::Png))
            }
        }
        ImageFormat::Webp => {
            #[cfg(feature = "webp")]
            {
                crate::formats::webp::decode_to_raw(input)
            }
            #[cfg(not(feature = "webp"))]
            {
                Err(ImgOptimError::not_built(ImageFormat::Webp))
            }
        }
    }
}

fn encode_from_raw(
    raw: &RawImage,
    fmt: ImageFormat,
    opts: &crate::formats::convert::OptimizeOptions,
    background: Option<[u8; 3]>,
) -> Result<Vec<u8>, ImgOptimError> {
    match fmt {
        ImageFormat::Jpeg => {
            #[cfg(feature = "jpeg")]
            {
                crate::formats::jpeg::encode_from_raw(raw, opts, background)
            }
            #[cfg(not(feature = "jpeg"))]
            {
                Err(ImgOptimError::not_built(ImageFormat::Jpeg))
            }
        }
        ImageFormat::Png => {
            #[cfg(feature = "png")]
            {
                crate::formats::png::encode_from_raw(raw, opts)
            }
            #[cfg(not(feature = "png"))]
            {
                Err(ImgOptimError::not_built(ImageFormat::Png))
            }
        }
        ImageFormat::Webp => {
            #[cfg(feature = "webp")]
            {
                crate::formats::webp::encode_from_raw(raw, opts)
            }
            #[cfg(not(feature = "webp"))]
            {
                Err(ImgOptimError::not_built(ImageFormat::Webp))
            }
        }
    }
}

fn apply_resize(
    raw: RawImage,
    spec: crate::rules::resize::ResizeSpec,
    fit: crate::cli::FitMode,
) -> RawImage {
    match raw.color {
        RawColor::Rgba8 => {
            let (w, h, pixels) =
                resize_rgba_bilinear(&raw.pixels, raw.width, raw.height, spec.w, spec.h, fit);
            RawImage {
                width: w,
                height: h,
                color: RawColor::Rgba8,
                pixels,
            }
        }
        RawColor::Rgb8 => {
            let (w, h, pixels) =
                resize_rgb_bilinear(&raw.pixels, raw.width, raw.height, spec.w, spec.h, fit);
            RawImage {
                width: w,
                height: h,
                color: RawColor::Rgb8,
                pixels,
            }
        }
        RawColor::L8 => {
            let rgb = raw
                .pixels
                .iter()
                .flat_map(|&v| [v, v, v])
                .collect::<Vec<u8>>();
            let (w, h, pixels) =
                resize_rgb_bilinear(&rgb, raw.width, raw.height, spec.w, spec.h, fit);
            RawImage {
                width: w,
                height: h,
                color: RawColor::Rgb8,
                pixels,
            }
        }
    }
}

fn encode_jpeg_to_target_size(
    raw: &RawImage,
    opts: &crate::formats::convert::OptimizeOptions,
    background: Option<[u8; 3]>,
    target_bytes: u64,
    max_quality: u8,
) -> Result<(Vec<u8>, bool), ImgOptimError> {
    let mut best = Vec::new();
    let mut reached = false;

    for q in (1..=max_quality).rev() {
        let mut o = opts.clone();
        o.quality = Some(q);
        let bytes = crate::formats::jpeg::encode_from_raw(raw, &o, background)?;
        if best.is_empty() || bytes.len() < best.len() {
            best = bytes;
        }
        if (best.len() as u64) <= target_bytes {
            reached = true;
            break;
        }
    }

    if best.is_empty() {
        best = crate::formats::jpeg::encode_from_raw(raw, opts, background)?;
    }

    Ok((best, reached))
}
