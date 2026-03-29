use crate::cli::StripSpec;
use crate::error::ImgOptimError;
use crate::formats::ImageFormat;

const XMP_APP1_ID: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const EXIF_APP1_ID: &[u8] = b"Exif\0\0";
const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const PNG_XMP_KEYWORD: &[u8] = b"XML:com.adobe.xmp";

fn build_xmp_packet(category: &str) -> Vec<u8> {
    fn esc(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    let cat = esc(category);

    let xml = format!(
        r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:xmp="http://ns.adobe.com/xap/1.0/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:dc="http://purl.org/dc/elements/1.1/">
   <dc:subject>
    <rdf:Bag>
     <rdf:li>{cat}</rdf:li>
    </rdf:Bag>
   </dc:subject>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
    );

    xml.into_bytes()
}

pub fn apply_tag_category(
    fmt: ImageFormat,
    input: &[u8],
    category: &str,
) -> Result<Vec<u8>, ImgOptimError> {
    match fmt {
        ImageFormat::Jpeg => replace_xmp_in_jpeg(input, category),
        ImageFormat::Png => replace_xmp_in_png(input, category),
        ImageFormat::Webp => inject_xmp_in_webp(input, category),
        ImageFormat::Tiff | ImageFormat::Jxl => Ok(input.to_vec()),
    }
}

pub fn has_exif(fmt: ImageFormat, input: &[u8]) -> bool {
    match fmt {
        ImageFormat::Jpeg => extract_jpeg_app1_payload(input, EXIF_APP1_ID).is_some(),
        ImageFormat::Png => crate::formats::png::read_png_metadata(input)
            .ok()
            .flatten()
            .and_then(|meta| meta.exif)
            .is_some(),
        ImageFormat::Webp => false,
        ImageFormat::Tiff | ImageFormat::Jxl => false,
    }
}

pub fn preserve_metadata(
    input_fmt: ImageFormat,
    output_fmt: ImageFormat,
    input: &[u8],
    output: &[u8],
) -> Result<Vec<u8>, ImgOptimError> {
    match (input_fmt, output_fmt) {
        (ImageFormat::Jpeg, ImageFormat::Jpeg) => {
            let exif = extract_jpeg_app1_payload(input, EXIF_APP1_ID);
            let xmp = extract_jpeg_app1_payload(input, XMP_APP1_ID);
            if exif.is_some() || xmp.is_some() {
                inject_jpeg_metadata(output, exif.as_deref(), xmp.as_deref())
            } else {
                Ok(output.to_vec())
            }
        }
        (ImageFormat::Png, ImageFormat::Jpeg) => {
            let meta = crate::formats::png::read_png_metadata(input)?;
            if let Some(meta) = meta {
                inject_jpeg_metadata(output, meta.exif.as_deref(), meta.xmp.as_deref())
            } else {
                Ok(output.to_vec())
            }
        }
        (ImageFormat::Jpeg, ImageFormat::Png) => {
            let exif = extract_jpeg_app1_payload(input, EXIF_APP1_ID);
            let xmp = extract_jpeg_app1_payload(input, XMP_APP1_ID);
            if exif.is_some() || xmp.is_some() {
                crate::formats::png::inject_png_meta(output, exif.as_deref(), None, xmp.as_deref())
            } else {
                Ok(output.to_vec())
            }
        }
        (ImageFormat::Webp, ImageFormat::Png) => {
            let xmp = extract_webp_xmp(input);
            if xmp.is_some() {
                crate::formats::png::inject_png_meta(output, None, None, xmp.as_deref())
            } else {
                Ok(output.to_vec())
            }
        }
        _ => Ok(output.to_vec()),
    }
}

pub fn strip_metadata(
    fmt: ImageFormat,
    input: &[u8],
    strip: &StripSpec,
) -> Result<Vec<u8>, ImgOptimError> {
    let any = strip.strip_all
        || strip.strip_exif
        || strip.strip_xmp
        || strip.strip_iptc
        || strip.strip_icc
        || strip.strip_com;
    if !any {
        return Ok(input.to_vec());
    }

    match fmt {
        ImageFormat::Jpeg => strip_jpeg_metadata(input, strip),
        ImageFormat::Png => strip_png_metadata(input, strip),
        _ => Ok(input.to_vec()),
    }
}

/* ---------------- JPEG ---------------- */

fn is_xmp_app1(seg_payload: &[u8]) -> bool {
    seg_payload.len() >= XMP_APP1_ID.len() && &seg_payload[..XMP_APP1_ID.len()] == XMP_APP1_ID
}

fn build_jpeg_xmp_app1(category: &str) -> Result<Vec<u8>, ImgOptimError> {
    let xmp = build_xmp_packet(category);

    let mut payload = Vec::with_capacity(XMP_APP1_ID.len() + xmp.len());
    payload.extend_from_slice(XMP_APP1_ID);
    payload.extend_from_slice(&xmp);

    let seg_len = payload.len() + 2;
    if seg_len > 0xffff {
        return Err(ImgOptimError::Processing(
            "XMP packet too large for APP1".into(),
        ));
    }

    let mut app1 = Vec::with_capacity(2 + 2 + payload.len());
    app1.extend_from_slice(&[0xff, 0xe1]);
    app1.extend_from_slice(&(seg_len as u16).to_be_bytes());
    app1.extend_from_slice(&payload);
    Ok(app1)
}

fn build_jpeg_app1(prefix: &[u8], payload: &[u8]) -> Result<Vec<u8>, ImgOptimError> {
    let seg_len = prefix.len() + payload.len() + 2;
    if seg_len > 0xffff {
        return Err(ImgOptimError::Processing("APP1 payload too large".into()));
    }
    let mut app1 = Vec::with_capacity(2 + 2 + prefix.len() + payload.len());
    app1.extend_from_slice(&[0xff, 0xe1]);
    app1.extend_from_slice(&(seg_len as u16).to_be_bytes());
    app1.extend_from_slice(prefix);
    app1.extend_from_slice(payload);
    Ok(app1)
}

fn is_app1_with_prefix(seg_payload: &[u8], prefix: &[u8]) -> bool {
    seg_payload.len() >= prefix.len() && &seg_payload[..prefix.len()] == prefix
}

fn extract_jpeg_app1_payload(input: &[u8], prefix: &[u8]) -> Option<Vec<u8>> {
    if input.len() < 4 || input[0] != 0xff || input[1] != 0xd8 {
        return None;
    }
    let mut i = 2usize;
    while i + 4 <= input.len() {
        if input[i] != 0xff {
            i += 1;
            continue;
        }
        while i < input.len() && input[i] == 0xff {
            i += 1;
        }
        if i >= input.len() {
            break;
        }
        let marker = input[i];
        i += 1;

        if marker == 0xd9 || marker == 0xda {
            break;
        }
        if i + 2 > input.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([input[i], input[i + 1]]) as usize;
        i += 2;
        if seg_len < 2 || i + (seg_len - 2) > input.len() {
            break;
        }
        let payload = &input[i..i + (seg_len - 2)];
        if marker == 0xe1 && is_app1_with_prefix(payload, prefix) {
            return Some(payload[prefix.len()..].to_vec());
        }
        i += seg_len - 2;
    }
    None
}

pub fn inject_jpeg_metadata(
    input: &[u8],
    exif: Option<&[u8]>,
    xmp: Option<&[u8]>,
) -> Result<Vec<u8>, ImgOptimError> {
    if input.len() < 2 || input[0] != 0xff || input[1] != 0xd8 {
        return Err(ImgOptimError::Processing(
            "invalid JPEG (missing SOI)".into(),
        ));
    }

    let mut segments = Vec::new();
    if let Some(exif) = exif {
        segments.push(build_jpeg_app1(EXIF_APP1_ID, exif)?);
    }
    if let Some(xmp) = xmp {
        segments.push(build_jpeg_app1(XMP_APP1_ID, xmp)?);
    }
    if segments.is_empty() {
        return Ok(input.to_vec());
    }

    let extra_len: usize = segments.iter().map(|s| s.len()).sum();
    let mut out = Vec::with_capacity(input.len() + extra_len);

    // Copy SOI and inject metadata
    out.extend_from_slice(&input[..2]);
    for s in segments {
        out.extend_from_slice(&s);
    }

    let mut i = 2usize;
    while i + 4 <= input.len() {
        if input[i] != 0xff {
            out.extend_from_slice(&input[i..]);
            return Ok(out);
        }

        while i < input.len() && input[i] == 0xff {
            i += 1;
        }
        if i >= input.len() {
            break;
        }

        let marker = input[i];
        i += 1;

        let standalone = marker == 0xd9 || (0xd0..=0xd7).contains(&marker) || marker == 0x01;
        if standalone {
            out.extend_from_slice(&[0xff, marker]);
            if marker == 0xd9 {
                return Ok(out);
            }
            continue;
        }

        if i + 2 > input.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([input[i], input[i + 1]]) as usize;
        i += 2;
        if seg_len < 2 || i + (seg_len - 2) > input.len() {
            break;
        }

        let payload = &input[i..i + (seg_len - 2)];
        if marker == 0xe1
            && (is_app1_with_prefix(payload, EXIF_APP1_ID)
                || is_app1_with_prefix(payload, XMP_APP1_ID))
        {
            i += seg_len - 2;
            continue;
        }

        out.extend_from_slice(&[0xff, marker]);
        out.extend_from_slice(&(seg_len as u16).to_be_bytes());
        out.extend_from_slice(payload);
        i += seg_len - 2;
    }

    Ok(out)
}

fn strip_jpeg_metadata(input: &[u8], strip: &StripSpec) -> Result<Vec<u8>, ImgOptimError> {
    if input.len() < 2 || input[0] != 0xff || input[1] != 0xd8 {
        return Err(ImgOptimError::Processing(
            "invalid JPEG (missing SOI)".into(),
        ));
    }

    let mut out = Vec::with_capacity(input.len());
    out.extend_from_slice(&input[..2]);

    let mut i = 2usize;
    while i + 4 <= input.len() {
        if input[i] != 0xff {
            out.extend_from_slice(&input[i..]);
            return Ok(out);
        }
        while i < input.len() && input[i] == 0xff {
            i += 1;
        }
        if i >= input.len() {
            break;
        }

        let marker = input[i];
        i += 1;

        let standalone = marker == 0xd9 || (0xd0..=0xd7).contains(&marker) || marker == 0x01;
        if standalone {
            out.extend_from_slice(&[0xff, marker]);
            if marker == 0xd9 {
                return Ok(out);
            }
            continue;
        }

        if i + 2 > input.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([input[i], input[i + 1]]) as usize;
        i += 2;
        if seg_len < 2 || i + (seg_len - 2) > input.len() {
            break;
        }

        let payload = &input[i..i + (seg_len - 2)];
        let mut skip = false;

        if marker == 0xe1 {
            if (strip.strip_all || strip.strip_exif) && is_app1_with_prefix(payload, EXIF_APP1_ID) {
                skip = true;
            }
            if (strip.strip_all || strip.strip_xmp) && is_app1_with_prefix(payload, XMP_APP1_ID) {
                skip = true;
            }
        }

        if marker == 0xe2
            && (strip.strip_all || strip.strip_icc)
            && payload.starts_with(b"ICC_PROFILE\0")
        {
            skip = true;
        }

        if marker == 0xed && (strip.strip_all || strip.strip_iptc) {
            skip = true;
        }

        if marker == 0xfe && (strip.strip_all || strip.strip_com) {
            skip = true;
        }

        if !skip {
            out.extend_from_slice(&[0xff, marker]);
            out.extend_from_slice(&(seg_len as u16).to_be_bytes());
            out.extend_from_slice(payload);
        }

        i += seg_len - 2;
    }

    Ok(out)
}

fn strip_png_metadata(input: &[u8], strip: &StripSpec) -> Result<Vec<u8>, ImgOptimError> {
    if input.len() < 8 || &input[..8] != PNG_SIG {
        return Err(ImgOptimError::Processing("invalid PNG signature".into()));
    }

    let mut out = Vec::with_capacity(input.len());
    out.extend_from_slice(&input[..8]);

    let mut pos = 8usize;
    while pos + 12 <= input.len() {
        let len = u32::from_be_bytes(input[pos..pos + 4].try_into().unwrap()) as usize;
        let ctype: [u8; 4] = input[pos + 4..pos + 8].try_into().unwrap();
        let chunk_total = 12 + len;

        if pos + chunk_total > input.len() {
            return Err(ImgOptimError::Processing("corrupt PNG chunk size".into()));
        }

        let data = &input[pos + 8..pos + 8 + len];

        let mut skip = false;

        if strip.strip_all {
            if ctype == *b"eXIf"
                || ctype == *b"iCCP"
                || ctype == *b"iTXt"
                || ctype == *b"tEXt"
                || ctype == *b"zTXt"
                || ctype == *b"sRGB"
                || ctype == *b"pHYs"
            {
                skip = true;
            }
        } else {
            if strip.strip_exif && ctype == *b"eXIf" {
                skip = true;
            }
            if strip.strip_icc && ctype == *b"iCCP" {
                skip = true;
            }
            if strip.strip_xmp && ctype == *b"iTXt" && itxt_keyword_is_xmp(data) {
                skip = true;
            }
        }

        if !skip {
            out.extend_from_slice(&input[pos..pos + chunk_total]);
        }

        if &ctype == b"IEND" {
            break;
        }

        pos += chunk_total;
    }

    Ok(out)
}

fn replace_xmp_in_jpeg(input: &[u8], category: &str) -> Result<Vec<u8>, ImgOptimError> {
    if input.len() < 2 || input[0] != 0xff || input[1] != 0xd8 {
        return Err(ImgOptimError::Processing(
            "invalid JPEG (missing SOI)".into(),
        ));
    }

    let new_app1 = build_jpeg_xmp_app1(category)?;

    // Copy SOI
    let mut out = Vec::with_capacity(input.len() + new_app1.len());
    out.extend_from_slice(&input[..2]);
    // Insert new XMP APP1 right after SOI
    out.extend_from_slice(&new_app1);

    // Then copy all other segments, skipping existing XMP APP1 segments
    let mut i = 2usize;

    while i + 4 <= input.len() {
        if input[i] != 0xff {
            // Not at a marker boundary => remaining is entropy-coded scan data; copy rest and stop
            out.extend_from_slice(&input[i..]);
            return Ok(out);
        }

        // Skip fill bytes 0xFF 0xFF... (rare but possible)
        while i < input.len() && input[i] == 0xff {
            i += 1;
        }
        if i >= input.len() {
            break;
        }

        let marker = input[i];
        i += 1;

        // Standalone markers (no length)
        // SOI(FFD8) handled; EOI(FFD9), RSTn(FFD0-FFD7), TEM(FF01)
        let standalone = marker == 0xd9 || (0xd0..=0xd7).contains(&marker) || marker == 0x01;

        if standalone {
            out.extend_from_slice(&[0xff, marker]);
            if marker == 0xd9 {
                // EOI: done
                return Ok(out);
            }
            continue;
        }

        // Markers with length
        if i + 2 > input.len() {
            return Err(ImgOptimError::Processing(
                "truncated JPEG segment length".into(),
            ));
        }
        let seg_len = u16::from_be_bytes([input[i], input[i + 1]]) as usize;
        i += 2;
        if seg_len < 2 {
            return Err(ImgOptimError::Processing(
                "invalid JPEG segment length".into(),
            ));
        }
        let payload_len = seg_len - 2;
        if i + payload_len > input.len() {
            return Err(ImgOptimError::Processing(
                "truncated JPEG segment payload".into(),
            ));
        }

        let payload = &input[i..i + payload_len];
        i += payload_len;

        // Skip XMP APP1
        if marker == 0xe1 && is_xmp_app1(payload) {
            continue;
        }

        // Otherwise copy segment as-is
        out.extend_from_slice(&[0xff, marker]);
        out.extend_from_slice(&(seg_len as u16).to_be_bytes());
        out.extend_from_slice(payload);

        // Start of Scan (SOS, FFDA): after this comes entropy data until EOI; copy remaining and stop
        if marker == 0xda {
            out.extend_from_slice(&input[i..]);
            return Ok(out);
        }
    }

    // If we fell out, copy any remaining bytes (defensive)
    if i < input.len() {
        out.extend_from_slice(&input[i..]);
    }

    Ok(out)
}

/* ---------------- WebP ---------------- */

fn extract_webp_xmp(input: &[u8]) -> Option<Vec<u8>> {
    if input.len() < 12 || &input[0..4] != b"RIFF" || &input[8..12] != b"WEBP" {
        return None;
    }

    let mut i = 12usize;
    while i + 8 <= input.len() {
        let fourcc = &input[i..i + 4];
        let size = u32::from_le_bytes(input[i + 4..i + 8].try_into().ok()?) as usize;
        let payload_start = i + 8;
        let payload_end = payload_start.checked_add(size)?;
        if payload_end > input.len() {
            return None;
        }

        if fourcc == b"XMP " {
            return Some(input[payload_start..payload_end].to_vec());
        }

        let padded = size + (size & 1);
        i = payload_start.checked_add(padded)?;
    }

    None
}

fn inject_xmp_in_webp(input: &[u8], category: &str) -> Result<Vec<u8>, ImgOptimError> {
    if input.len() < 12 || &input[0..4] != b"RIFF" || &input[8..12] != b"WEBP" {
        return Err(ImgOptimError::Processing("invalid WebP signature".into()));
    }

    let xmp = build_xmp_packet(category);
    let mut out = Vec::with_capacity(input.len() + xmp.len() + 16);
    out.extend_from_slice(&input[..12]);

    let mut i = 12usize;
    while i + 8 <= input.len() {
        let fourcc = &input[i..i + 4];
        let size = u32::from_le_bytes(input[i + 4..i + 8].try_into().unwrap()) as usize;
        let payload_start = i + 8;
        let payload_end = payload_start + size;
        if payload_end > input.len() {
            return Err(ImgOptimError::Processing("corrupt WebP chunk size".into()));
        }

        if fourcc != b"XMP " {
            out.extend_from_slice(&input[i..payload_end]);
            if size & 1 == 1 {
                if let Some(pad) = input.get(payload_end) {
                    out.push(*pad);
                }
            }
        }

        let padded = size + (size & 1);
        i = payload_start + padded;
    }

    out.extend_from_slice(b"XMP ");
    out.extend_from_slice(&(xmp.len() as u32).to_le_bytes());
    out.extend_from_slice(&xmp);
    if xmp.len() & 1 == 1 {
        out.push(0);
    }

    let riff_size = (out.len() - 8) as u32;
    out[4..8].copy_from_slice(&riff_size.to_le_bytes());

    Ok(out)
}

/* ---------------- PNG ---------------- */

fn make_png_chunk(ctype: [u8; 4], data: &[u8]) -> Vec<u8> {
    use crc32fast::Hasher;

    let mut out = Vec::with_capacity(12 + data.len());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&ctype);
    out.extend_from_slice(data);

    let mut h = Hasher::new();
    h.update(&ctype);
    h.update(data);
    out.extend_from_slice(&h.finalize().to_be_bytes());

    out
}

fn build_png_itxt_xmp(category: &str) -> Vec<u8> {
    let xmp = build_xmp_packet(category);

    // iTXt:
    // keyword\0 + compression_flag + compression_method + language_tag\0 + translated_keyword\0 + text
    let mut data = Vec::new();
    data.extend_from_slice(PNG_XMP_KEYWORD);
    data.push(0);
    data.push(0); // uncompressed
    data.push(0); // method
    data.push(0); // language tag empty
    data.push(0); // translated keyword empty
    data.extend_from_slice(&xmp);

    make_png_chunk(*b"iTXt", &data)
}

fn itxt_keyword_is_xmp(data: &[u8]) -> bool {
    // keyword is a null-terminated string at start
    let nul = match data.iter().position(|&b| b == 0) {
        Some(p) => p,
        None => {
            return false;
        }
    };
    &data[..nul] == PNG_XMP_KEYWORD
}

fn replace_xmp_in_png(input: &[u8], category: &str) -> Result<Vec<u8>, ImgOptimError> {
    if input.len() < 8 || &input[..8] != PNG_SIG {
        return Err(ImgOptimError::Processing("invalid PNG signature".into()));
    }

    let new_itxt = build_png_itxt_xmp(category);

    let mut out = Vec::with_capacity(input.len() + new_itxt.len());
    out.extend_from_slice(&input[..8]);

    let mut pos = 8usize;

    while pos + 12 <= input.len() {
        let len = u32::from_be_bytes(input[pos..pos + 4].try_into().unwrap()) as usize;
        let ctype: [u8; 4] = input[pos + 4..pos + 8].try_into().unwrap();
        let chunk_total = 12 + len;

        if pos + chunk_total > input.len() {
            return Err(ImgOptimError::Processing("corrupt PNG chunk size".into()));
        }

        let data = &input[pos + 8..pos + 8 + len];
        let inserted = false;

        // Insert before IEND
        if &ctype == b"IEND" {
            if !inserted {
                out.extend_from_slice(&new_itxt);
            }
            out.extend_from_slice(&input[pos..pos + chunk_total]);
            return Ok(out);
        }

        // Skip existing XMP iTXt
        if &ctype == b"iTXt" && itxt_keyword_is_xmp(data) {
            pos += chunk_total;
            continue;
        }

        // Copy chunk as-is
        out.extend_from_slice(&input[pos..pos + chunk_total]);
        pos += chunk_total;
    }

    Err(ImgOptimError::Processing("PNG missing IEND chunk".into()))
}
