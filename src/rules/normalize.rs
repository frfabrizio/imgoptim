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
            "`--jpeg-normal` and `--jpeg-progressive` are mutually exclusive".into(),
        ));
    }

    if o.zopfli_iteration_count.is_some()
        || o.zopfli_max_block_splits.is_some()
        || o.zopfli_timeout_secs.is_some()
    {
        o.zopfli = true;
    }

    if let Some(v) = o.zopfli_iteration_count {
        if v == 0 {
            return Err(ImgOptimError::InvalidArgs(
                "--zopfli-iteration-count must be >= 1".into(),
            ));
        }
    }
    if let Some(v) = o.zopfli_timeout_secs {
        if v == 0 {
            return Err(ImgOptimError::InvalidArgs(
                "--zopfli-timeout must be >= 1".into(),
            ));
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CommonOpts;

    fn default_common() -> CommonOpts {
        CommonOpts {
            dest: None,
            overwrite: false,
            preserve: false,
            noaction: false,
            force: false,
            threshold_percent: None,
            target_size: None,
            totals: false,
            quiet: false,
            verbose: false,
            output_format: None,
            name_suffix: None,
            keep_ext: false,
            inplace: false,
            only: Vec::new(),
            skip: Vec::new(),
            strip_all: false,
            strip_exif: false,
            strip_xmp: false,
            strip_iptc: false,
            strip_icc: false,
            strip_com: false,
            keep_metadata: false,
            tag_category: None,
            max_quality: None,
            quality: None,
            all_normal: false,
            all_progressive: false,
            jpeg_turbo: false,
            jpeg_sampling: None,
            png_level: None,
            zopfli: false,
            zopfli_iteration_count: None,
            zopfli_max_block_splits: None,
            zopfli_timeout_secs: None,
            webp_lossless: false,
            convert_input: None,
            convert_lossless: false,
            convert_lossy: false,
            convert_background: None,
            convert_resize: None,
            convert_fit: None,
        }
    }

    #[test]
    fn zopfli_options_enable_flag() {
        let mut c = default_common();
        c.zopfli_iteration_count = Some(1);
        let mut o = Opts::from_common(c);
        o.inputs = vec![std::path::PathBuf::from("input.png")];
        let out = normalize_options(Mode::Optimize, o).expect("normalize");
        assert!(out.zopfli, "zopfli should be enabled");
    }

    #[test]
    fn zopfli_iteration_count_zero_is_rejected() {
        let mut c = default_common();
        c.zopfli_iteration_count = Some(0);
        let mut o = Opts::from_common(c);
        o.inputs = vec![std::path::PathBuf::from("input.png")];
        let err = normalize_options(Mode::Optimize, o).unwrap_err();
        assert!(
            err.to_string().contains("zopfli-iteration-count"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn zopfli_timeout_zero_is_rejected() {
        let mut c = default_common();
        c.zopfli_timeout_secs = Some(0);
        let mut o = Opts::from_common(c);
        o.inputs = vec![std::path::PathBuf::from("input.png")];
        let err = normalize_options(Mode::Optimize, o).unwrap_err();
        assert!(
            err.to_string().contains("zopfli-timeout"),
            "unexpected error: {err}"
        );
    }
}
