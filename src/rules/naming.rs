use std::path::{Path, PathBuf};

use crate::cli::{Mode, Opts};
use crate::error::ImgOptimError;
use crate::formats::ImageFormat;

fn output_ext(fmt: ImageFormat) -> &'static str {
    match fmt {
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Png => "png",
        ImageFormat::Webp => "webp",
    }
}

/// Build output path according to:
/// - --dest
/// - --name-suffix
/// - mode optimize vs convert
/// - convert: --inplace / --keep-ext
pub fn make_output_path(
    input_path: &Path,
    input_fmt: ImageFormat,
    opts: &Opts,
) -> Result<PathBuf, ImgOptimError> {
    // Directory
    let dir = match &opts.dest {
        Some(d) => d.clone(),
        None => input_path
            .parent()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| ImgOptimError::InvalidArgs("input has no parent directory".into()))?,
    };

    // Base name (file stem)
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ImgOptimError::InvalidArgs("invalid input filename".into()))?;

    // Choose output format/extension
    let out_fmt = match opts.mode {
        Mode::Optimize => input_fmt,
        Mode::Convert => {
            let cv = opts
                .convert
                .as_ref()
                .ok_or_else(|| ImgOptimError::InvalidArgs("convert options missing".into()))?;
            match cv.output {
                crate::cli::Fmt::Jpeg => ImageFormat::Jpeg,
                crate::cli::Fmt::Png => ImageFormat::Png,
                crate::cli::Fmt::Webp => ImageFormat::Webp,
            }
        }
    };

    // Suffix
    let suffix = opts.name_suffix.as_deref().unwrap_or("");

    // Determine extension rules (convert only)
    let (final_stem, final_ext) = match opts.mode {
        Mode::Optimize => (format!("{stem}{suffix}"), output_ext(out_fmt).to_string()),

        Mode::Convert => {
            if opts.keep_ext {
                // keep original extension AND require suffix (or implicit)
                let in_ext = input_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("img");

                let suf = if suffix.is_empty() { ".conv" } else { suffix };
                (format!("{stem}{suf}"), in_ext.to_string())
            } else {
                // inplace behavior = extension becomes output format by default
                // If --inplace false but no --dest, still "inplace" semantics are OK.
                // Here we just always use output extension.
                (format!("{stem}{suffix}"), output_ext(out_fmt).to_string())
            }
        }
    };

    Ok(dir.join(format!("{final_stem}.{final_ext}")))
}
