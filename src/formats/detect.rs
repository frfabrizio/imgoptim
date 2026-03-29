use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use super::ImageFormat;

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
/// - TIFF: "II*\0" or "MM\0*"
/// - JXL: container signature box or codestream signature
pub fn detect_format(path: &Path) -> io::Result<Option<ImageFormat>> {
    let head = read_head(path, 64)?;

    Ok(detect_format_from_bytes(&head))
}

pub fn detect_format_from_bytes(head: &[u8]) -> Option<ImageFormat> {
    // JPEG
    if head.len() >= 3 && head[0] == 0xff && head[1] == 0xd8 && head[2] == 0xff {
        return Some(ImageFormat::Jpeg);
    }

    // PNG
    const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if head.len() >= 8 && head[..8] == PNG_SIG {
        return Some(ImageFormat::Png);
    }

    // WebP: RIFF....WEBP
    if head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        return Some(ImageFormat::Webp);
    }

    // TIFF: II*\0 or MM\0*
    if head.len() >= 4
        && ((head[0..4] == [0x49, 0x49, 0x2a, 0x00]) || (head[0..4] == [0x4d, 0x4d, 0x00, 0x2a]))
    {
        return Some(ImageFormat::Tiff);
    }

    // JXL codestream signature: FF 0A
    if head.len() >= 2 && head[0] == 0xff && head[1] == 0x0a {
        return Some(ImageFormat::Jxl);
    }

    // JXL container signature box: 00 00 00 0C 4A 58 4C 20 0D 0A 87 0A
    const JXL_CONTAINER_SIG: [u8; 12] = [
        0x00, 0x00, 0x00, 0x0c, 0x4a, 0x58, 0x4c, 0x20, 0x0d, 0x0a, 0x87, 0x0a,
    ];
    if head.len() >= 12 && head[..12] == JXL_CONTAINER_SIG {
        return Some(ImageFormat::Jxl);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::detect_format_from_bytes;
    use crate::formats::ImageFormat;

    #[test]
    fn detect_tiff_little_endian_magic() {
        let bytes = [0x49, 0x49, 0x2a, 0x00, 0x08, 0x00];
        assert_eq!(detect_format_from_bytes(&bytes), Some(ImageFormat::Tiff));
    }

    #[test]
    fn detect_tiff_big_endian_magic() {
        let bytes = [0x4d, 0x4d, 0x00, 0x2a, 0x00, 0x08];
        assert_eq!(detect_format_from_bytes(&bytes), Some(ImageFormat::Tiff));
    }

    #[test]
    fn detect_jxl_codestream_magic() {
        let bytes = [0xff, 0x0a, 0x00, 0x01];
        assert_eq!(detect_format_from_bytes(&bytes), Some(ImageFormat::Jxl));
    }

    #[test]
    fn detect_jxl_container_magic() {
        let bytes = [
            0x00, 0x00, 0x00, 0x0c, 0x4a, 0x58, 0x4c, 0x20, 0x0d, 0x0a, 0x87, 0x0a,
        ];
        assert_eq!(detect_format_from_bytes(&bytes), Some(ImageFormat::Jxl));
    }
}
