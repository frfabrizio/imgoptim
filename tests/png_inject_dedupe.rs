use imgoptim::formats::png::inject_png_meta;

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

fn count_chunks(png: &[u8], want: &[u8; 4]) -> usize {
    if png.len() < 8 {
        return 0;
    }
    let mut pos = 8usize;
    let mut n = 0usize;
    while pos + 12 <= png.len() {
        let len = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
        let ctype: [u8; 4] = png[pos + 4..pos + 8].try_into().unwrap();
        if &ctype == want {
            n += 1;
        }
        pos += 12 + len;
    }
    n
}

#[test]
fn inject_png_meta_dedupes_exif_icc_xmp() {
    // Minimal PNG structure: signature + IHDR + IEND (CRC ok)
    let mut png = Vec::new();
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // IHDR: 13 bytes
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(1u32).to_be_bytes()); // width
    ihdr.extend_from_slice(&(1u32).to_be_bytes()); // height
    ihdr.push(8); // bit depth
    ihdr.push(2); // color type RGB
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    png.extend_from_slice(&make_png_chunk(*b"IHDR", &ihdr));

    // Add pre-existing eXIf, iCCP, and XMP iTXt (to ensure they get removed)
    png.extend_from_slice(&make_png_chunk(*b"eXIf", b"OLD_EXIF"));

    // Fake iCCP layout (still syntactically correct; payload doesn't have to be meaningful here)
    let mut iccp = Vec::new();
    iccp.extend_from_slice(b"ICC Profile");
    iccp.push(0);
    iccp.push(0);
    iccp.extend_from_slice(b"\x78\x9c\x03\x00\x00\x00\x00\x01"); // tiny zlib stream (may be nonsense, but chunk exists)
    png.extend_from_slice(&make_png_chunk(*b"iCCP", &iccp));

    // XMP iTXt keyword
    let mut itxt = Vec::new();
    itxt.extend_from_slice(b"XML:com.adobe.xmp");
    itxt.push(0);
    itxt.push(0);
    itxt.push(0);
    itxt.push(0);
    itxt.push(0);
    itxt.extend_from_slice(b"<xmp>old</xmp>");
    png.extend_from_slice(&make_png_chunk(*b"iTXt", &itxt));

    // IEND
    png.extend_from_slice(&make_png_chunk(*b"IEND", &[]));

    // Inject new meta (should dedupe old and insert one of each)
    let out = inject_png_meta(
        &png,
        Some(b"NEW_EXIF"),
        Some(b"NEW_ICC_PROFILE_BYTES"),
        Some(b"<xmp>new</xmp>"),
    )
    .unwrap();

    assert_eq!(count_chunks(&out, b"eXIf"), 1);
    assert_eq!(count_chunks(&out, b"iCCP"), 1);
    assert_eq!(count_chunks(&out, b"iTXt"), 1);

    // Inject again: should remain one each
    let out2 = inject_png_meta(
        &out,
        Some(b"NEW_EXIF2"),
        Some(b"NEW_ICC_PROFILE_BYTES2"),
        Some(b"<xmp>new2</xmp>"),
    )
    .unwrap();

    assert_eq!(count_chunks(&out2, b"eXIf"), 1);
    assert_eq!(count_chunks(&out2, b"iCCP"), 1);
    assert_eq!(count_chunks(&out2, b"iTXt"), 1);
}
