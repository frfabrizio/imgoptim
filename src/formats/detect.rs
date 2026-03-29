use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use super::ImageFormat;

const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Read the first N bytes of a file (N up to 64 here).
fn read_head(path: &Path, n: usize) -> io::Result<Vec<u8>> {
    let mut f = File::open(path)?;
    let mut buf = vec![0u8; n];
    let mut read = 0usize;

    while read < n {
        let r = f.read(&mut buf[read..])?;
        if r == 0 {
            break;
        }
        read += r;
    }
    buf.truncate(read);
    Ok(buf)
}

/// Detect image format by magic bytes (not by extension).
///
/// Recognizes:
/// - JPEG: FF D8 FF
/// - PNG:  89 50 4E 47 0D 0A 1A 0A
/// - WebP: "RIFF" .... "WEBP"
/// - TIFF: "II*\0", "MM\0*", plus BigTIFF variants
/// - JXL: codestream "FF 0A" or container signature "JXL "
///
/// # Errors
///
/// Returns any I/O error encountered while reading the file header.
pub fn detect_format(path: &Path) -> io::Result<Option<ImageFormat>> {
    let head = read_head(path, 64)?;

    Ok(detect_format_from_bytes(&head))
}

#[must_use]
pub fn detect_format_from_bytes(head: &[u8]) -> Option<ImageFormat> {
    // JPEG
    if head.len() >= 3 && head[0] == 0xff && head[1] == 0xd8 && head[2] == 0xff {
        return Some(ImageFormat::Jpeg);
    }

    // PNG
    if head.len() >= 8 && head[..8] == PNG_SIG {
        return Some(ImageFormat::Png);
    }

    // WebP: RIFF....WEBP
    if head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        return Some(ImageFormat::Webp);
    }

    // TIFF classic + BigTIFF
    if head.len() >= 4
        && ((head[0] == b'I' && head[1] == b'I' && (head[2], head[3]) == (0x2a, 0x00))
            || (head[0] == b'M' && head[1] == b'M' && (head[2], head[3]) == (0x00, 0x2a))
            || (head[0] == b'I' && head[1] == b'I' && (head[2], head[3]) == (0x2b, 0x00))
            || (head[0] == b'M' && head[1] == b'M' && (head[2], head[3]) == (0x00, 0x2b)))
    {
        return Some(ImageFormat::Tiff);
    }

    // JPEG XL codestream (FF 0A)
    if head.len() >= 2 && head[0] == 0xff && head[1] == 0x0a {
        return Some(ImageFormat::Jxl);
    }
    // JPEG XL container signature: 00 00 00 0C 'J' 'X' 'L' ' ' 0D 0A 87 0A
    if head.len() >= 12
        && head[0..4] == [0x00, 0x00, 0x00, 0x0c]
        && &head[4..8] == b"JXL "
        && head[8..12] == [0x0d, 0x0a, 0x87, 0x0a]
    {
        return Some(ImageFormat::Jxl);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tiff_little_endian() {
        let bytes = [b'I', b'I', 0x2a, 0x00, 0x08, 0x00, 0x00, 0x00];
        assert_eq!(detect_format_from_bytes(&bytes), Some(ImageFormat::Tiff));
    }

    #[test]
    fn detects_jxl_codestream() {
        let bytes = [0xff, 0x0a, 0x00, 0x11];
        assert_eq!(detect_format_from_bytes(&bytes), Some(ImageFormat::Jxl));
    }
}
