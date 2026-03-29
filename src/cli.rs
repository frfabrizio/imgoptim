use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "imgoptim",
    version,
    about = "Multi-format image optimizer/converter (JPEG/PNG/WebP + TIFF/JXL input) inspired by jpegoptim",
    disable_help_subcommand = true,
    arg_required_else_help = true
)]
pub struct Cmd {
    #[command(flatten)]
    pub common: CommonOpts,

    #[command(subcommand)]
    pub sub: Option<Sub>,

    /// Input filenames (optimize mode)
    #[arg(value_name = "filenames", trailing_var_arg = true)]
    pub inputs: Vec<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Sub {
    /// Convert images to another format
    Convert(ConvertCmd),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Optimize,
    Convert,
}

impl Cmd {
    pub fn into_mode_and_options(self) -> (Mode, Opts) {
        match self.sub {
            None => {
                let mut o = Opts::from_common(self.common);
                o.mode = Mode::Optimize;
                o.inputs = self.inputs;
                (Mode::Optimize, o)
            }
            Some(Sub::Convert(c)) => {
                let mut o = Opts::from_common(self.common);
                o.mode = Mode::Convert;
                o.inputs = c.inputs.clone();
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
    Tiff,
    Jxl,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMode {
    Contain,
    Cover,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

impl Verbosity {
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

    pub png_level: Option<u8>,
    pub zopfli: bool,

    pub webp_lossless: bool,
    pub webp_method: Option<u8>,

    // Convert
    pub convert: Option<ConvertOpts>,

    // Inputs
    pub inputs: Vec<PathBuf>,
}

#[derive(Parser, Debug, Clone)]
pub struct CommonOpts {
    #[arg(short = 'd', long = "dest")]
    pub dest: Option<PathBuf>,

    #[arg(short = 'o', long = "overwrite")]
    pub overwrite: bool,

    #[arg(short = 'p', long = "preserve")]
    pub preserve: bool,

    #[arg(short = 'n', long = "noaction")]
    pub noaction: bool,

    #[arg(short = 'f', long = "force")]
    pub force: bool,

    #[arg(short = 'T', long = "threshold")]
    pub threshold_percent: Option<f32>,

    #[arg(short = 'S', long = "size")]
    pub target_size: Option<String>,

    #[arg(short = 't', long = "totals")]
    pub totals: bool,

    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    #[arg(long = "name-suffix")]
    pub name_suffix: Option<String>,

    /// Keep original extension (convert mode: requires suffix).
    #[arg(long = "keep-ext")]
    pub keep_ext: bool,

    /// Replace extension in destination (default if --dest not set).
    #[arg(long = "inplace")]
    pub inplace: bool,

    #[arg(long = "only", value_enum)]
    pub only: Vec<Fmt>,

    #[arg(long = "skip", value_enum)]
    pub skip: Vec<Fmt>,

    #[arg(long = "strip-all")]
    pub strip_all: bool,
    #[arg(long = "strip-exif")]
    pub strip_exif: bool,
    #[arg(long = "strip-xmp")]
    pub strip_xmp: bool,
    #[arg(long = "strip-iptc")]
    pub strip_iptc: bool,
    #[arg(long = "strip-icc")]
    pub strip_icc: bool,
    #[arg(long = "strip-com")]
    pub strip_com: bool,
    #[arg(long = "keep-metadata")]
    pub keep_metadata: bool,

    #[arg(long = "tag-category")]
    pub tag_category: Option<String>,

    /// JPEG max quality (lossy).
    #[arg(short = 'm', long = "max")]
    pub max_quality: Option<u8>,

    /// Generic quality (format-specific).
    #[arg(long = "quality")]
    pub quality: Option<u8>,

    /// WebP: not supported (lossless only).
    #[arg(long = "webp-quality")]
    pub webp_quality: Option<u8>,

    #[arg(long = "all-normal", conflicts_with = "all_progressive")]
    pub all_normal: bool,

    #[arg(long = "all-progressive", conflicts_with = "all_normal")]
    pub all_progressive: bool,

    /// Prefer libjpeg-turbo when available.
    #[arg(long = "jpeg-turbo")]
    pub jpeg_turbo: bool,

    /// PNG compression level 0..9 (default: 6).
    #[arg(long = "png-level")]
    pub png_level: Option<u8>,

    /// Use Zopfli (very slow, best compression).
    #[arg(long = "zopfli")]
    pub zopfli: bool,

    /// Force lossless WebP encoding (only supported mode).
    #[arg(long = "webp-lossless")]
    pub webp_lossless: bool,

    /// WebP: not supported (lossless only).
    #[arg(long = "webp-method")]
    pub webp_method: Option<u8>,
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

    /// Resize images (WxH, Wx, xH).
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

    pub fn from_common(c: CommonOpts) -> Self {
        let verbosity = Self::verbosity_from_flags(c.quiet, c.verbose);
        let mut quality = c.quality;
        if quality.is_none() {
            quality = c.webp_quality;
        }

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

            png_level: c.png_level,
            zopfli: c.zopfli,

            webp_lossless: c.webp_lossless,
            webp_method: c.webp_method,

            convert: None,
            inputs: Vec::new(),
        }
    }
}
