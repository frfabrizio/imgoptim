mod common;

mod jpeg_to_png {
    use super::common::*;

    #[test]
    fn jpeg_to_png_with_tag_category_injects_xmp() {
        let out_dir = tmp_out_dir();
        let input = asset_path("tests/assets/jpeg/photo_meta.jpg");
        assert!(input.exists(), "missing asset: {}", input.display());

        run_ok(&[
            "--keep-metadata",
            "--tag-category",
            "Optimis?",
            "--name-suffix",
            "_imgoptim",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            "--output-format",
            "png",
            input.to_str().unwrap(),
        ]);

        let out = out_dir.path().join("photo_meta_imgoptim.png");
        expect_file_exists(&out);

        let bytes = read_bytes(&out);
        assert_is_png(&bytes);

        let xmp = png_extract_xmp(&bytes).expect("XMP not found in PNG");
        let xmp_s = String::from_utf8_lossy(&xmp);
        assert!(
            xmp_s.contains("http://ns.adobe.com/xap/1.0/") || xmp_s.contains("xmpmeta"),
            "XMP does not look like XMP"
        );
        assert!(xmp_s.contains("Optimis?"), "tag-category not found in XMP");
    }
}

#[cfg(feature = "webp")]
mod jpeg_to_webp {
    use super::common::*;

    #[test]
    fn jpeg_to_webp_without_metadata_has_no_xmp() {
        let out_dir = tmp_out_dir();
        let input = asset_path("tests/assets/jpeg/photo_nometa.jpg");
        assert!(input.exists(), "missing asset: {}", input.display());

        run_ok(&[
            "--name-suffix",
            "_imgoptim",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            "--output-format",
            "webp",
            input.to_str().unwrap(),
        ]);

        let out = out_dir.path().join("photo_nometa_imgoptim.webp");
        expect_file_exists(&out);

        let bytes = read_bytes(&out);
        assert_is_webp(&bytes);

        assert!(
            !webp_contains_xmp(&bytes),
            "unexpected XMP found in WebP converted from no-meta JPEG"
        );
    }

    #[test]
    fn jpeg_to_webp_with_metadata_and_tag_category_injects_xmp() {
        let out_dir = tmp_out_dir();
        let input = asset_path("tests/assets/jpeg/photo_meta.jpg");
        assert!(input.exists(), "missing asset: {}", input.display());

        run_ok(&[
            "--keep-metadata",
            "--tag-category",
            "Optimis??",
            "--name-suffix",
            "_imgoptim",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            "--output-format",
            "webp",
            input.to_str().unwrap(),
        ]);

        let out = out_dir.path().join("photo_meta_imgoptim.webp");
        expect_file_exists(&out);

        let bytes = read_bytes(&out);
        assert_is_webp(&bytes);

        assert!(
            webp_contains_xmp(&bytes),
            "expected XMP to be present in WebP when using --tag-category"
        );
    }
}

mod png_to_jpeg {
    use super::common::*;

    #[test]
    fn png_to_jpeg_writes_and_injects_meta_with_keep_metadata() {
        let out_dir = tmp_out_dir();
        let input = asset_path("tests/assets/png/image_meta.png");
        assert!(input.exists(), "missing asset: {}", input.display());

        run_ok(&[
            "--keep-metadata",
            "--name-suffix",
            "_imgoptim",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            "--output-format",
            "jpeg",
            "--background",
            "#ffffff",
            input.to_str().unwrap(),
        ]);

        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("input file_stem must be valid utf-8");

        let out = expect_jpeg_out(out_dir.path(), &format!("{stem}_imgoptim"));

        let bytes = read_bytes(&out);
        assert_is_jpeg(&bytes);

        assert!(
            jpeg_contains_exif(&bytes) || jpeg_extract_xmp(&bytes).is_some(),
            "expected EXIF or XMP to be present in JPEG when --keep-metadata"
        );
    }

    #[test]
    fn png_to_jpeg_preserves_meta_by_default() {
        let out_dir = tmp_out_dir();
        let input = asset_path("tests/assets/png/image_meta.png");
        assert!(input.exists(), "missing asset: {}", input.display());

        run_ok(&[
            "--name-suffix",
            "_imgoptim",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            "--output-format",
            "jpeg",
            "--background",
            "#ffffff",
            input.to_str().unwrap(),
        ]);

        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("input file_stem must be valid utf-8");

        let out = expect_jpeg_out(out_dir.path(), &format!("{stem}_imgoptim"));

        let bytes = read_bytes(&out);
        assert_is_jpeg(&bytes);

        assert!(
            jpeg_contains_exif(&bytes) || jpeg_extract_xmp(&bytes).is_some(),
            "expected EXIF or XMP to be present in JPEG by default"
        );
    }

    #[test]
    fn png_to_jpeg_strip_all_removes_metadata() {
        let out_dir = tmp_out_dir();
        let input = asset_path("tests/assets/png/image_meta.png");
        assert!(input.exists(), "missing asset: {}", input.display());

        run_ok(&[
            "--strip-all",
            "--name-suffix",
            "_imgoptim",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            "--output-format",
            "jpeg",
            "--background",
            "#ffffff",
            input.to_str().unwrap(),
        ]);

        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("input file_stem must be valid utf-8");

        let out = expect_jpeg_out(out_dir.path(), &format!("{stem}_imgoptim"));

        let bytes = read_bytes(&out);
        assert_is_jpeg(&bytes);

        assert!(
            !jpeg_contains_exif(&bytes),
            "unexpected EXIF found in JPEG with --strip-all"
        );
        assert!(
            jpeg_extract_xmp(&bytes).is_none(),
            "unexpected XMP found in JPEG with --strip-all"
        );
    }

    #[test]
    fn png_to_jpeg_with_tag_category_injects_xmp() {
        let out_dir = tmp_out_dir();
        let input = asset_path("tests/assets/png/image_nometa.png");
        assert!(input.exists(), "missing asset: {}", input.display());

        run_ok(&[
            "--keep-metadata",
            "--tag-category",
            "Optimis??",
            "--name-suffix",
            "_imgoptim",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            "--output-format",
            "jpeg",
            "--background",
            "#ffffff",
            input.to_str().unwrap(),
        ]);

        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("input file_stem must be valid utf-8");

        let out = expect_jpeg_out(out_dir.path(), &format!("{stem}_imgoptim"));

        let bytes = read_bytes(&out);
        assert_is_jpeg(&bytes);

        let xmp = jpeg_extract_xmp(&bytes).expect("XMP not found in JPEG");
        let xmp_s = String::from_utf8_lossy(&xmp);

        assert!(
            xmp_s.contains("Optimis??"),
            "tag-category not found in JPEG XMP"
        );
    }
}

#[cfg(all(feature = "webp", feature = "png"))]
mod png_to_webp {
    use super::common::*;

    #[test]
    fn png_to_webp_writes_file() {
        let out_dir = tmp_out_dir();
        let input = asset_path("tests/assets/png/image_nometa.png");
        assert!(input.exists(), "missing asset: {}", input.display());

        run_ok(&[
            "--name-suffix",
            "_imgoptim",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            "--output-format",
            "webp",
            input.to_str().unwrap(),
        ]);

        let out = out_dir.path().join("image_nometa_imgoptim.webp");
        expect_file_exists(&out);

        let bytes = read_bytes(&out);
        assert_is_webp(&bytes);
    }

    #[test]
    fn png_to_webp_with_tag_category_injects_xmp() {
        let out_dir = tmp_out_dir();
        let input = asset_path("tests/assets/png/image_nometa.png");
        assert!(input.exists(), "missing asset: {}", input.display());

        run_ok(&[
            "--keep-metadata",
            "--tag-category",
            "Optimis?",
            "--name-suffix",
            "_imgoptim",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            "--output-format",
            "webp",
            input.to_str().unwrap(),
        ]);

        let out = out_dir.path().join("image_nometa_imgoptim.webp");
        expect_file_exists(&out);

        let bytes = read_bytes(&out);
        assert_is_webp(&bytes);

        assert!(
            webp_contains_xmp(&bytes),
            "expected XMP to be present in WebP when using --tag-category"
        );
    }
}

#[cfg(all(feature = "webp", feature = "png", feature = "jpeg"))]
mod webp_to_png {
    use super::common::*;

    #[test]
    fn webp_to_png_roundtrip_preserves_tag_category_as_png_xmp() {
        let out_dir = tmp_out_dir();

        let input_jpeg = asset_path("tests/assets/jpeg/photo_nometa.jpg");
        assert!(
            input_jpeg.exists(),
            "missing asset: {}",
            input_jpeg.display()
        );

        run_ok(&[
            "--keep-metadata",
            "--tag-category",
            "Optimis?",
            "--name-suffix",
            "_imgoptim",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            "--output-format",
            "webp",
            input_jpeg.to_str().unwrap(),
        ]);

        let webp_out = out_dir.path().join("photo_nometa_imgoptim.webp");
        expect_file_exists(&webp_out);

        run_ok(&[
            "--name-suffix",
            "_imgoptim2",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            "--output-format",
            "png",
            webp_out.to_str().unwrap(),
        ]);

        let png_out = out_dir.path().join("photo_nometa_imgoptim_imgoptim2.png");
        expect_file_exists(&png_out);

        let png_bytes = read_bytes(&png_out);
        assert_is_png(&png_bytes);

        let xmp =
            png_extract_xmp(&png_bytes).expect("XMP not found in PNG after WebP->PNG roundtrip");
        let xmp_s = String::from_utf8_lossy(&xmp);
        assert!(
            xmp_s.contains("Optimis?"),
            "tag-category not found in PNG XMP"
        );
    }
}
