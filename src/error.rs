use std::fmt;
use std::io;
use crate::formats;

/// Type de résultat applicatif pour imgoptim
pub type ResultError<T> = std::result::Result<T, ImgOptimError>;

#[derive(Debug)]
pub enum ImgOptimError {
    Io(io::Error),
    Processing(String),
    UnknownFormat,
    UnsupportedFormat(String),
    InvalidArgs(String),
    InvalidOption(String),
    Metadata(String),
    NotBuilt { detected: formats::ImageFormat },
}

impl ImgOptimError {
    pub fn not_built(detected: formats::ImageFormat) -> Self {Self::NotBuilt { detected } }
    pub fn processing(msg: impl Into<String>) -> Self {Self::Processing(msg.into()) }
}

impl fmt::Display for ImgOptimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {}", err),
            Self::Processing(msg) => write!(f, "Processing error: {msg}"),
            Self::UnknownFormat => write!(f, "Unknown format (could not detect input format)"),
            Self::UnsupportedFormat(fmt) => write!(f, "Unsupported format: {}", fmt),
            Self::InvalidArgs(msg) => write!(f, "Invalid arguments: {}", msg),
            Self::InvalidOption(msg) => write!(f, "Invalid option: {}", msg),
            Self::Metadata(msg) => write!(f, "Metadata error: {}", msg),
            Self::NotBuilt { detected } => write!(
                f,
                "Support not built for detected format/feature: {}",
                detected
            ),
        }
    }
}

impl std::error::Error for ImgOptimError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

/* ---------- From conversions ---------- */

impl From<io::Error> for ImgOptimError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
