use assert_cmd::assert::Assert;
use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

#[allow(dead_code)]
pub fn imgoptim_cmd() -> Command {
    cargo_bin_cmd!("imgoptim")
}

#[allow(dead_code)]
pub fn tmp_out_dir() -> TempDir {
    tempdir().expect("failed to create temp dir")
}

#[allow(dead_code)]
pub fn asset_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[allow(dead_code)]
pub fn read_bytes(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[allow(dead_code)]
pub fn expect_file_exists(path: &Path) {
    assert!(path.exists(), "expected file to exist: {}", path.display());
}

#[allow(dead_code)]
pub fn assert_is_png(bytes: &[u8]) {
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"), "not a PNG");
}

#[allow(dead_code)]
pub fn assert_is_jpeg(bytes: &[u8]) {
    assert!(
        bytes.len() >= 4
            && bytes[0] == 0xFF
            && bytes[1] == 0xD8
            && bytes[bytes.len() - 2] == 0xFF
            && bytes[bytes.len() - 1] == 0xD9,
        "not a JPEG"
    );
}

#[allow(dead_code)]
pub fn assert_is_webp(bytes: &[u8]) {
    assert!(bytes.len() >= 12, "not a WebP (too small)");
    assert!(&bytes[0..4] == b"RIFF", "not a WebP (missing RIFF)");
    assert!(&bytes[8..12] == b"WEBP", "not a WebP (missing WEBP)");
}

/// ---------------------------
/// PNG XMP extractor (iTXt)
/// Looks for iTXt keyword "XML:com.adobe.xmp" (common XMP-in-PNG convention)
/// ---------------------------
#[allow(dead_code)]
pub fn png_extract_xmp(bytes: &[u8]) -> Option<Vec<u8>> {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let mut i = 8usize; // after signature
    while i + 12 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[i..i + 4].try_into().ok()?) as usize;
        let ctype = &bytes[i + 4..i + 8];
        let data_start = i + 8;
        let data_end = data_start.checked_add(len)?;
        let crc_end = data_end.checked_add(4)?;
        if crc_end > bytes.len() {
            return None;
        }

        if ctype == b"iTXt" {
            let data = &bytes[data_start..data_end];
            if let Some(xmp) = parse_png_itxt_xmp(data) {
                return Some(xmp);
            }
        }

        if ctype == b"IEND" {
            break;
        }
        i = crc_end;
    }
    None
}

#[allow(dead_code)]
fn parse_png_itxt_xmp(data: &[u8]) -> Option<Vec<u8>> {
    // iTXt layout:
    // keyword\0 compression_flag compression_method language_tag\0 translated_keyword\0 text

    // keyword\0
    let nul = data.iter().position(|&b| b == 0)?;
    let keyword = &data[..nul];
    if keyword != b"XML:com.adobe.xmp" {
        return None;
    }
    let mut p = nul + 1;

    // compression_flag + compression_method
    let compression_flag = *data.get(p)?;
    let _compression_method = *data.get(p + 1)?;
    p += 2;

    // language_tag\0
    let nul2 = data.get(p..)?.iter().position(|&b| b == 0)?;
    p += nul2 + 1;

    // translated_keyword\0
    let nul3 = data.get(p..)?.iter().position(|&b| b == 0)?;
    p += nul3 + 1;

    // text
    let text = data.get(p..)?;
    if compression_flag == 0 {
        Some(text.to_vec())
    } else {
        // Compressed iTXt not supported in test parser
        None
    }
}

/// ---------------------------
/// JPEG segment scanners (EXIF / XMP)
/// ---------------------------
#[allow(dead_code)]
pub fn jpeg_contains_exif(bytes: &[u8]) -> bool {
    jpeg_extract_app1_payload(bytes, b"Exif\0\0").is_some()
}

#[allow(dead_code)]
pub fn jpeg_extract_xmp(bytes: &[u8]) -> Option<Vec<u8>> {
    // XMP APP1 prefix is "http://ns.adobe.com/xap/1.0/\0"
    let prefix = b"http://ns.adobe.com/xap/1.0/\0";
    jpeg_extract_app1_payload(bytes, prefix)
}

#[allow(dead_code)]
fn jpeg_extract_app1_payload(bytes: &[u8], prefix: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }
    let mut i = 2usize;
    while i + 4 <= bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        // skip fill bytes
        while i < bytes.len() && bytes[i] == 0xFF {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let marker = bytes[i];
        i += 1;

        // markers without length
        if marker == 0xD9 || marker == 0xDA {
            break;
        }
        if i + 2 > bytes.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        i += 2;
        if seg_len < 2 || i + (seg_len - 2) > bytes.len() {
            break;
        }
        let payload = &bytes[i..i + (seg_len - 2)];
        if marker == 0xE1 && payload.starts_with(prefix) {
            return Some(payload[prefix.len()..].to_vec());
        }
        i += seg_len - 2;
    }
    None
}

/// ---------------------------
/// WebP XMP detector (RIFF chunk "XMP ")
/// ---------------------------
#[allow(dead_code)]
pub fn webp_contains_xmp(bytes: &[u8]) -> bool {
    // RIFF....WEBP
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return false;
    }

    let mut i = 12;
    while i + 8 <= bytes.len() {
        let fourcc = &bytes[i..i + 4];

        let size_bytes: [u8; 4] = match bytes.get(i + 4..i + 8).and_then(|s| s.try_into().ok()) {
            Some(v) => v,
            None => return false,
        };
        let size = u32::from_le_bytes(size_bytes) as usize;

        let payload_start = i + 8;
        let payload_end = match payload_start.checked_add(size) {
            Some(v) => v,
            None => return false,
        };
        if payload_end > bytes.len() {
            return false;
        }

        // WebP extended chunks: "XMP " chunk contains XMP packet
        if fourcc == b"XMP " {
            let payload = &bytes[payload_start..payload_end];
            // On cherche juste un marqueur robuste
            return payload
                .windows(b"http://ns.adobe.com/xap/1.0/".len())
                .any(|w| w == b"http://ns.adobe.com/xap/1.0/");
        }

        // Chunks are padded to even size
        let padded = size + (size & 1);
        i = match payload_start.checked_add(padded) {
            Some(v) => v,
            None => return false,
        };
    }

    false
}

/// Convenience: run command and assert success
#[allow(dead_code)]
pub fn run_ok(args: &[&str]) -> Assert {
    let mut cmd = imgoptim_cmd();
    cmd.args(args).assert().success()
}

#[allow(dead_code)]
pub fn run_ok_with_input(args: &[&str], input: &str) -> Assert {
    let mut cmd = imgoptim_cmd();
    cmd.args(args).write_stdin(input).assert().success()
}

#[allow(dead_code)]
pub fn expect_jpeg_out(dest: &Path, stem_with_suffix: &str) -> PathBuf {
    let candidates = [
        format!("{stem_with_suffix}.jpg"),
        format!("{stem_with_suffix}.jpeg"),
        format!("{stem_with_suffix}.JPG"),
        format!("{stem_with_suffix}.JPEG"),
    ];

    for name in &candidates {
        let p = dest.join(name);
        if p.is_file() {
            return p;
        }
    }

    let mut seen = Vec::new();
    if let Ok(rd) = fs::read_dir(dest) {
        for e in rd.flatten() {
            if let Some(n) = e.file_name().to_str() {
                seen.push(n.to_string());
            }
        }
    }

    panic!(
        "expected JPEG output not found in {}\nexpected one of: {:?}\nseen: {:?}",
        dest.display(),
        candidates,
        seen
    );
}
