use clap::Parser;
use std::io::Write;
use tracing::warn;

use imgoptim::cli::Cmd;
use imgoptim::rules::decision;
use imgoptim::rules::normalize::normalize_options;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let cmd = Cmd::parse();
    let (mode, opts) = cmd.into_mode_and_options();

    let opts = normalize_options(mode, opts)?;
    let is_quiet = matches!(opts.verbosity, imgoptim::cli::Verbosity::Quiet);
    let is_verbose = opts.verbosity.is_verbose();

    if is_verbose {
        if let Some(dest) = opts.dest.as_ref() {
            println!("Destination directory: {}", dest.display());
        }
        if let Some(max) = opts.max_quality {
            println!("Image quality limit set to: {max}");
        }
        if let Some(quality) = opts.quality {
            println!("Image quality set to: {quality}");
        }
        let options = format_selected_options(mode, &opts);
        if !options.is_empty() {
            println!("Options: {}", options.join(" "));
        }
    }

    let mut had_error = false;

    let inputs = expand_inputs(&opts.inputs);
    let total = inputs.len();
    let mut processed = 0usize;
    for input in inputs.iter() {
        let input_path = input.as_path();

        match decision::process_one(input_path, &opts) {
            Ok(summary) => {
                if !is_quiet {
                    println!("{summary}");
                }
            }
            Err(e) => {
                had_error = true;
                decision::record_error();
                if !matches!(opts.verbosity, imgoptim::cli::Verbosity::Quiet) {
                    // tracing "structuré" => plus robuste, évite les soucis d'inférence
                    warn!(path = %input_path.display(), error = %e, "processing failed");
                }
            }
        }

        processed += 1;
        if is_quiet {
            print_progress(processed, total);
        }
    }

    if is_quiet && total > 0 {
        println!();
    }

    if opts.print_totals {
        decision::print_totals();
    }

    if had_error {
        std::process::exit(2);
    }
    Ok(())
}

fn expand_inputs(inputs: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for input in inputs {
        let raw = input.to_string_lossy();
        if has_glob_chars(&raw) {
            let mut matched = false;
            let options = glob::MatchOptions {
                case_sensitive: false,
                require_literal_separator: false,
                require_literal_leading_dot: false,
            };
            let pattern = raw.replace('\\', "/");
            if let Ok(paths) = glob::glob_with(&pattern, options) {
                for entry in paths.flatten() {
                    out.push(entry);
                    matched = true;
                }
            }
            if !matched {
                out.push(input.clone());
            }
        } else {
            out.push(input.clone());
        }
    }
    out
}

fn has_glob_chars(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

fn print_progress(done: usize, total: usize) {
    let pct = if total == 0 {
        100
    } else {
        (done * 100) / total
    };
    print!("\rProgress: {done}/{total} ({pct}%)");
    let _ = std::io::stdout().flush();
}

fn format_selected_options(mode: imgoptim::cli::Mode, opts: &imgoptim::cli::Opts) -> Vec<String> {
    let mut out = Vec::new();

    if opts.overwrite {
        out.push("--overwrite".to_string());
    }
    if opts.preserve {
        out.push("--preserve".to_string());
    }
    if opts.dry_run {
        out.push("--noaction".to_string());
    }
    if opts.force {
        out.push("--force".to_string());
    }
    if let Some(fmt) = opts.output_format {
        out.push(format!("--output-format={}", fmt_to_str(fmt)));
    }
    if let Some(v) = opts.threshold_percent {
        out.push(format!("--threshold={v}"));
    }
    if let Some(v) = opts.target_size.as_deref() {
        out.push(format!("--size={v}"));
    }
    if opts.print_totals {
        out.push("--totals".to_string());
    }
    if let Some(v) = opts.name_suffix.as_deref() {
        out.push(format!("--name-suffix={v}"));
    }
    if opts.keep_ext {
        out.push("--keep-ext".to_string());
    }
    if opts.inplace {
        out.push("--inplace".to_string());
    }
    for f in &opts.only {
        out.push(format!("--only={}", fmt_to_str(*f)));
    }
    for f in &opts.skip {
        out.push(format!("--skip={}", fmt_to_str(*f)));
    }

    if opts.strip.strip_all {
        out.push("--strip-all".to_string());
    }
    if opts.strip.strip_exif {
        out.push("--strip-exif".to_string());
    }
    if opts.strip.strip_xmp {
        out.push("--strip-xmp".to_string());
    }
    if opts.strip.strip_iptc {
        out.push("--strip-iptc".to_string());
    }
    if opts.strip.strip_icc {
        out.push("--strip-icc".to_string());
    }
    if opts.strip.strip_com {
        out.push("--strip-com".to_string());
    }
    if opts.strip.keep_metadata {
        out.push("--keep-metadata".to_string());
    }
    if let Some(v) = opts.tag_category.as_deref() {
        out.push(format!("--tag-category={v}"));
    }

    if opts.all_normal {
        out.push("--jpeg-normal".to_string());
    }
    if opts.all_progressive {
        out.push("--jpeg-progressive".to_string());
    }
    if opts.jpeg_turbo {
        out.push("--jpeg-turbo".to_string());
    }
    if let Some(v) = opts.png_level {
        out.push(format!("--png-level={v}"));
    }
    if opts.zopfli {
        out.push("--png-zopfli".to_string());
    }
    if let Some(v) = opts.zopfli_iteration_count {
        out.push(format!("--zopfli-iteration-count={v}"));
    }
    if let Some(v) = opts.zopfli_max_block_splits {
        out.push(format!("--zopfli-max-block-splits={v}"));
    }
    if let Some(v) = opts.zopfli_timeout_secs {
        out.push(format!("--zopfli-timeout={v}"));
    }
    if opts.webp_lossless {
        out.push("--webp-lossless".to_string());
    }

    if mode == imgoptim::cli::Mode::Convert {
        if let Some(cv) = opts.convert.as_ref() {
            if opts.output_format.is_none() {
                out.push(format!("--output={}", fmt_to_str(cv.output)));
            }
            if let Some(v) = cv.input {
                out.push(format!("--input={}", fmt_to_str(v)));
            }
            if cv.lossless {
                out.push("--lossless".to_string());
            }
            if cv.lossy {
                out.push("--lossy".to_string());
            }
            out.push(format!("--background={}", cv.background));
            if let Some(v) = cv.resize.as_deref() {
                out.push(format!("--resize={v}"));
            }
            out.push(format!("--fit={}", fit_to_str(cv.fit)));
        }
    }

    out
}

fn fmt_to_str(fmt: imgoptim::cli::Fmt) -> &'static str {
    match fmt {
        imgoptim::cli::Fmt::Jpeg => "jpeg",
        imgoptim::cli::Fmt::Png => "png",
        imgoptim::cli::Fmt::Webp => "webp",
    }
}

fn fit_to_str(fit: imgoptim::cli::FitMode) -> &'static str {
    match fit {
        imgoptim::cli::FitMode::Contain => "contain",
        imgoptim::cli::FitMode::Cover => "cover",
        imgoptim::cli::FitMode::Stretch => "stretch",
    }
}
