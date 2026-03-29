use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "imgoptim",
    version,
    about = "Multi-format image optimizer/converter (JPEG/PNG/WebP) inspired by jpegoptim",
    disable_help_subcommand = true,
    arg_required_else_help = true
)]
pub struct Cmd {
    #[command(flatten)]
    pub common: CommonOpts,

    #[command(subcommand)]
    pub sub: Option<Sub>,

    /// Input filenames (optimize mode)
    #[arg(value_name = "filenames")]
    pub inputs: Vec<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Sub {
    /// Convert images to another format
    #[command(hide = true)]
    Convert(ConvertCmd),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Optimize,
    Convert,
}

impl Cmd {
    #[must_use]
    pub fn into_mode_and_options(self) -> (Mode, Opts) {
        match self.sub {
            None => {
                let common = self.common;
                let mut o = Opts::from_common(common.clone());
                if let Some(fmt) = o.output_format {
                    o.mode = Mode::Convert;
                    o.convert = Some(ConvertOpts::from_common_output(fmt, &common));
                } else {
                    o.mode = Mode::Optimize;
                }
                o.inputs = self.inputs;
                (o.mode, o)
            }
            Some(Sub::Convert(c)) => {
                let mut o = Opts::from_common(self.common);
                o.mode = Mode::Convert;
                o.inputs.clone_from(&c.inputs);
                o.convert = Some(ConvertOpts {
                    output: c.output,
                    input: c.input,
                    lossless: c.lossless,
                    lossy: c.lossy,
                    background: c.background,
                    resize: c.resize,
                    fit: c.fit,
                });
                (Mode::Convert, o)
            }
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fmt {
    Jpeg,
    Png,
    Webp,
}
fn parse_fmt_ci(s: &str) -> Result<Fmt, String> {
    match s.to_ascii_lowercase().as_str() {
        "jpeg" | "jpg" => Ok(Fmt::Jpeg),
        "png" => Ok(Fmt::Png),
        "webp" => Ok(Fmt::Webp),
        _ => Err("expected one of: jpeg, png, webp".to_string()),
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMode {
    Contain,
    Cover,
    Stretch,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum JpegSampling {
    #[value(name = "444")]
    S444,
    #[value(name = "422")]
    S422,
    #[value(name = "420")]
    S420,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

impl Verbosity {
    #[must_use]
    pub fn is_verbose(self) -> bool {
        matches!(self, Verbosity::Verbose)
    }
}

#[derive(Debug, Clone, Default)]
pub struct StripSpec {
    pub keep_metadata: bool,
    pub strip_all: bool,
    pub strip_exif: bool,
    pub strip_xmp: bool,
    pub strip_iptc: bool,
    pub strip_icc: bool,
    pub strip_com: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSize {
    KiloBytes(u64),
    Percent(u8),
}

#[derive(Debug, Clone)]
pub struct ConvertOpts {
    pub output: Fmt,
    pub input: Option<Fmt>,
    pub lossless: bool,
    #[allow(dead_code)]
    pub lossy: bool,
    pub background: String,
    pub resize: Option<String>,
    pub fit: FitMode,
}

impl ConvertOpts {
    fn from_common_output(fmt: Fmt, c: &CommonOpts) -> Self {
        Self {
            output: fmt,
            input: c.convert_input,
            lossless: c.convert_lossless,
            lossy: c.convert_lossy,
            background: c
                .convert_background
                .clone()
                .unwrap_or_else(|| "#ffffff".to_string()),
            resize: c.convert_resize.clone(),
            fit: c.convert_fit.unwrap_or(FitMode::Contain),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Opts {
    pub mode: Mode,

    // Global
    pub dest: Option<PathBuf>,
    pub overwrite: bool,
    pub preserve: bool,
    pub dry_run: bool,
    pub force: bool,
    pub threshold_percent: Option<f32>,
    pub target_size: Option<String>,
    pub target_size_parsed: Option<TargetSize>,
    pub print_totals: bool,
    pub verbosity: Verbosity,

    // Naming
    pub name_suffix: Option<String>,
    pub keep_ext: bool,
    pub inplace: bool,

    // Filtering
    pub only: Vec<Fmt>,
    pub skip: Vec<Fmt>,

    // Metadata
    pub strip: StripSpec,
    pub tag_category: Option<String>,

    // Quality / jpegoptim compat
    pub max_quality: Option<u8>,
    pub quality: Option<u8>,

    // Progressive flags
    pub all_normal: bool,
    pub all_progressive: bool,
    pub jpeg_turbo: bool,
    pub jpeg_sampling: Option<JpegSampling>,

    pub png_level: Option<u8>,
    pub zopfli: bool,
    pub zopfli_iteration_count: Option<u64>,
    pub zopfli_max_block_splits: Option<u16>,
    pub zopfli_timeout_secs: Option<u64>,

    pub webp_lossless: bool,
    pub webp_method: Option<u8>,

    // Output format (convert without subcommand)
    pub output_format: Option<Fmt>,

    // Convert
    pub convert: Option<ConvertOpts>,

    // Inputs
    pub inputs: Vec<PathBuf>,
}

#[derive(Parser, Debug, Clone)]
pub struct CommonOpts {
    /// Alternative destination directory (default: overwrite input).
    #[arg(short = 'd', long = "dest")]
    pub dest: Option<PathBuf>,

    /// Overwrite target file even if it exists.
    #[arg(short = 'o', long = "overwrite")]
    pub overwrite: bool,

    /// Preserve file timestamps.
    #[arg(short = 'p', long = "preserve")]
    pub preserve: bool,

    /// Dry-run: don't write files, just print results.
    #[arg(short = 'n', long = "noaction")]
    pub noaction: bool,

    /// Force processing (even if it may not reduce size).
    #[arg(short = 'f', long = "force")]
    pub force: bool,

    /// Keep old file if gain is below threshold (%).
    #[arg(short = 'T', long = "threshold")]
    pub threshold_percent: Option<f32>,

    /// Try to reach target size (KB or %).
    #[arg(short = 'S', long = "size")]
    pub target_size: Option<String>,

    /// Print totals after processing all files.
    #[arg(short = 't', long = "totals")]
    pub totals: bool,

    /// Quiet mode.
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Verbose mode.
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Convert to output format without using subcommand (case-insensitive).
    #[arg(
        long = "output-format",
        alias = "output",
        value_parser = parse_fmt_ci
    )]
    pub output_format: Option<Fmt>,

    /// Append suffix before extension (e.g. "_imgoptim").
    #[arg(long = "name-suffix")]
    pub name_suffix: Option<String>,

    /// Keep original extension (convert mode: requires suffix).
    #[arg(long = "keep-ext")]
    pub keep_ext: bool,

    /// Replace extension in destination (default if --dest not set).
    #[arg(long = "inplace")]
    pub inplace: bool,

    /// Process only given format(s): jpeg,png,webp (repeatable).
    #[arg(long = "only", value_enum)]
    pub only: Vec<Fmt>,

    /// Skip given format(s): jpeg,png,webp (repeatable).
    #[arg(long = "skip", value_enum)]
    pub skip: Vec<Fmt>,

    /// Remove all metadata (EXIF/XMP/IPTC/ICC/COM as applicable).
    #[arg(long = "strip-all")]
    pub strip_all: bool,
    /// Remove EXIF metadata.
    #[arg(long = "strip-exif")]
    pub strip_exif: bool,
    /// Remove XMP metadata.
    #[arg(long = "strip-xmp")]
    pub strip_xmp: bool,
    /// Remove IPTC metadata.
    #[arg(long = "strip-iptc")]
    pub strip_iptc: bool,
    /// Remove ICC profile.
    #[arg(long = "strip-icc")]
    pub strip_icc: bool,
    /// Remove JPEG comment markers (COM).
    #[arg(long = "strip-com")]
    pub strip_com: bool,
    /// Override strip options (keep everything).
    #[arg(long = "keep-metadata")]
    pub keep_metadata: bool,

    /// Add a category tag when supported (e.g. XMP subject).
    #[arg(long = "tag-category")]
    pub tag_category: Option<String>,

    /// JPEG max quality (lossy).
    #[arg(short = 'm', long = "max")]
    pub max_quality: Option<u8>,

    /// JPEG quality 0..100 (only for JPEG output).
    #[arg(long = "quality")]
    pub quality: Option<u8>,

    /// JPEG: force baseline (non-progressive).
    #[arg(long = "jpeg-normal", conflicts_with = "all_progressive")]
    pub all_normal: bool,

    /// JPEG: force progressive encoding.
    #[arg(long = "jpeg-progressive", conflicts_with = "all_normal")]
    pub all_progressive: bool,

    /// Prefer libjpeg-turbo when available.
    #[arg(long = "jpeg-turbo")]
    pub jpeg_turbo: bool,

    /// JPEG chroma subsampling: 444, 422 or 420.
    #[arg(long = "jpeg-sampling", value_enum)]
    pub jpeg_sampling: Option<JpegSampling>,

    /// PNG compression level 0..9 (default: 6).
    #[arg(long = "png-level")]
    pub png_level: Option<u8>,

    /// Use Zopfli (very slow, best compression).
    #[arg(long = "png-zopfli")]
    pub zopfli: bool,

    /// Zopfli: max number of iterations (recommended: 1..15).
    #[arg(long = "zopfli-iteration-count")]
    pub zopfli_iteration_count: Option<u64>,

    /// Zopfli: maximum block splits (0 = unlimited, recommended: 0..15).
    #[arg(long = "zopfli-max-block-splits")]
    pub zopfli_max_block_splits: Option<u16>,

    /// Zopfli: timeout in seconds (recommended: 10..60).
    #[arg(long = "zopfli-timeout", alias = "zopfli--timeout")]
    pub zopfli_timeout_secs: Option<u64>,

    /// Force lossless WebP encoding (only supported mode).
    #[arg(long = "webp-lossless")]
    pub webp_lossless: bool,

    // Convert options (used with --output-format)
    /// Convert only files matching input format (optional).
    #[arg(long = "input", value_enum)]
    pub convert_input: Option<Fmt>,

    /// Request lossless conversion when possible.
    #[arg(long = "lossless", conflicts_with = "convert_lossy")]
    pub convert_lossless: bool,

    /// Allow lossy conversion when needed.
    #[arg(long = "lossy", conflicts_with = "convert_lossless")]
    pub convert_lossy: bool,

    /// Background color for alpha removal when converting to JPEG (e.g. #ffffff).
    #[arg(long = "background")]
    pub convert_background: Option<String>,

    /// Resize images (`WxH`, `Wx`, `xH`).
    #[arg(long = "resize")]
    pub convert_resize: Option<String>,

    /// Resize fit mode: contain, cover, stretch.
    #[arg(long = "fit", value_enum)]
    pub convert_fit: Option<FitMode>,
}

#[derive(Parser, Debug, Clone)]
pub struct ConvertCmd {
    #[arg(long = "output", value_enum)]
    pub output: Fmt,

    #[arg(long = "input", value_enum)]
    pub input: Option<Fmt>,

    #[arg(long = "lossless", conflicts_with = "lossy")]
    pub lossless: bool,

    #[arg(long = "lossy", conflicts_with = "lossless")]
    pub lossy: bool,

    #[arg(long = "background", default_value = "#ffffff")]
    pub background: String,

    /// Resize images (`WxH`, `Wx`, `xH`).
    #[arg(long = "resize")]
    pub resize: Option<String>,

    /// Resize fit mode: contain, cover, stretch.
    #[arg(long = "fit", value_enum, default_value = "contain")]
    pub fit: FitMode,

    /// Input filenames (convert mode)
    #[arg(value_name = "filenames", required = true)]
    pub inputs: Vec<PathBuf>,
}

impl Opts {
    fn verbosity_from_flags(quiet: bool, verbose: bool) -> Verbosity {
        if quiet {
            Verbosity::Quiet
        } else if verbose {
            Verbosity::Verbose
        } else {
            Verbosity::Normal
        }
    }

    #[must_use]
    pub fn from_common(c: CommonOpts) -> Self {
        let verbosity = Self::verbosity_from_flags(c.quiet, c.verbose);
        let quality = c.quality;

        Self {
            mode: Mode::Optimize,
            dest: c.dest,
            overwrite: c.overwrite,
            preserve: c.preserve,
            dry_run: c.noaction,
            force: c.force,
            threshold_percent: c.threshold_percent,
            target_size: c.target_size,
            target_size_parsed: None,
            print_totals: c.totals,
            verbosity,

            name_suffix: c.name_suffix,
            keep_ext: c.keep_ext,
            inplace: c.inplace,

            only: c.only,
            skip: c.skip,

            strip: StripSpec {
                keep_metadata: c.keep_metadata,
                strip_all: c.strip_all,
                strip_exif: c.strip_exif,
                strip_xmp: c.strip_xmp,
                strip_iptc: c.strip_iptc,
                strip_icc: c.strip_icc,
                strip_com: c.strip_com,
            },
            tag_category: c.tag_category,

            max_quality: c.max_quality,
            quality,

            all_normal: c.all_normal,
            all_progressive: c.all_progressive,
            jpeg_turbo: c.jpeg_turbo,
            jpeg_sampling: c.jpeg_sampling,

            png_level: c.png_level,
            zopfli: c.zopfli,
            zopfli_iteration_count: c.zopfli_iteration_count,
            zopfli_max_block_splits: c.zopfli_max_block_splits,
            zopfli_timeout_secs: c.zopfli_timeout_secs,

            webp_lossless: c.webp_lossless,
            webp_method: None,

            output_format: c.output_format,

            convert: None,
            inputs: Vec::new(),
        }
    }
}
