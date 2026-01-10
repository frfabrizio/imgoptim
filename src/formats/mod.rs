//! Central module for image formats.
//!
//! This module exposes:
//! - format-specific modules: `jpeg`, `png`, `webp` (1 file each, per your refactor)
//! - cross-format utilities: `detect`, `convert`, `resize`, metadata helpers, etc.
//! - the `ImageFormat` enum + helpers
//!
//! Format availability is controlled by Cargo features:
//! - `jpeg`
//! - `png`
//! - `webp`

pub mod convert;
pub mod detect;
pub mod metadata;
pub mod resize;
pub mod xmp;

// One unified module per format (your new design).
// Gate each one behind its Cargo feature to keep compilation clean.
#[cfg(feature = "jpeg")]
pub mod jpeg;

#[cfg(feature = "png")]
pub mod png;

#[cfg(feature = "webp")]
pub mod webp;

use std::fmt;
use std::path::Path;

/// Supported image formats for this crate.
///
/// Note: "supported" can mean:
/// - recognized by the code (`ImageFormat`)
/// - and optionally "built into the binary" (see `is_built`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
}

impl ImageFormat {
    /// Stable lowercase identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "jpeg",
            ImageFormat::Png => "png",
            ImageFormat::Webp => "webp",
        }
    }

    /// Common filename extensions (lowercase, no dot).
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            ImageFormat::Jpeg => &["jpg", "jpeg", "jpe"],
            ImageFormat::Png => &["png"],
            ImageFormat::Webp => &["webp"],
        }
    }

    /// Try to parse from a file extension (with or without leading dot).
    ///
    /// Examples: "jpg", ".jpeg", "PNG" (case-insensitive).
    pub fn from_extension(ext: &str) -> Option<Self> {
        let ext = ext.trim().trim_start_matches('.').to_ascii_lowercase();
        match ext.as_str() {
            "jpg" | "jpeg" | "jpe" => Some(ImageFormat::Jpeg),
            "png" => Some(ImageFormat::Png),
            "webp" => Some(ImageFormat::Webp),
            _ => None,
        }
    }

    /// Try to infer from a path extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|s| s.to_str())
            .and_then(Self::from_extension)
    }

    /// Try to parse from common MIME types.
    pub fn from_mime(mime: &str) -> Option<Self> {
        let m = mime.trim().to_ascii_lowercase();
        match m.as_str() {
            "image/jpeg" | "image/jpg" => Some(ImageFormat::Jpeg),
            "image/png" => Some(ImageFormat::Png),
            "image/webp" => Some(ImageFormat::Webp),
            _ => None,
        }
    }
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Strict policy A: a format is supported only if it is recognized AND built into the binary.
///
/// Uses Cargo features:
/// - feature "jpeg"
/// - feature "png"
/// - feature "webp"
pub fn is_built(fmt: ImageFormat) -> bool {
    match fmt {
        ImageFormat::Jpeg => cfg!(feature = "jpeg"),
        ImageFormat::Png => cfg!(feature = "png"),
        ImageFormat::Webp => cfg!(feature = "webp"),
    }
}
