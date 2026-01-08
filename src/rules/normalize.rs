use crate::cli::{Mode, Opts};
use crate::error::ImgOptimError;
use crate::rules::size::parse_target_size;

pub fn normalize_options(mode: Mode, mut o: Opts) -> Result<Opts, ImgOptimError> {
    // --- Conflits CLI ---
    if matches!(mode, Mode::Optimize) && o.inputs.is_empty() {
        return Err(ImgOptimError::InvalidArgs("no input files provided".into()));
    }
    if o.keep_ext && o.inplace {
        return Err(ImgOptimError::InvalidArgs(
            "`--keep-ext` and `--inplace` are mutually exclusive".into(),
        ));
    }

    // --- Bornes qualité ---
    if let Some(q) = o.max_quality {
        if q > 100 {
            return Err(ImgOptimError::InvalidArgs("--max must be 0..100".into()));
        }
    }
    if let Some(q) = o.quality {
        if q > 100 {
            return Err(ImgOptimError::InvalidArgs(
                "--quality must be 0..100".into(),
            ));
        }
    }

    // --- Bornes threshold ---
    if let Some(t) = o.threshold_percent {
        if !(0.0..=100.0).contains(&t) {
            return Err(ImgOptimError::InvalidArgs(
                "--threshold must be 0..100".into(),
            ));
        }
    }

    // --- Parse --size into structured form ---
    if let Some(s) = o.target_size.as_deref() {
        o.target_size_parsed = Some(parse_target_size(s)?);
    }

    // --- Priorité keep-metadata > strip-* ---
    if o.strip.keep_metadata {
        o.strip.strip_all = false;
        o.strip.strip_exif = false;
        o.strip.strip_xmp = false;
        o.strip.strip_iptc = false;
        o.strip.strip_icc = false;
        o.strip.strip_com = false;
    }

    if o.all_normal && o.all_progressive {
        return Err(ImgOptimError::InvalidArgs(
            "`--all-normal` and `--all-progressive` are mutually exclusive".into(),
        ));
    }

    // --- Policy A : convert fail-fast si formats demandés non compilés ---
    if matches!(mode, Mode::Convert) {
        let cv = o.convert.as_ref().ok_or_else(|| {
            ImgOptimError::InvalidArgs("convert mode requires convert options".into())
        })?;

        // v0.1: --size in convert is supported only for JPEG output (quality search)
        if o.target_size_parsed.is_some() && cv.output != crate::cli::Fmt::Jpeg {
            return Err(ImgOptimError::InvalidArgs(
                "--size in convert mode is only supported with --output jpeg (v0.1)".into(),
            ));
        }

        // v0.1: lossless conversion to JPEG is impossible
        if cv.lossless && cv.output == crate::cli::Fmt::Jpeg {
            return Err(ImgOptimError::InvalidArgs(
                "--lossless cannot be used with --output jpeg".into(),
            ));
        }
    }

    Ok(o)
}
