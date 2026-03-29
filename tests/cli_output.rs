mod common;

use common::*;

use std::fs;

#[test]
fn glob_expands_inputs() {
    let input_dir = tmp_out_dir();
    let out_dir = tmp_out_dir();
    let src = asset_path("tests/assets/jpeg/photo_nometa.jpg");

    let a = input_dir.path().join("a.jpg");
    let b = input_dir.path().join("b.jpg");
    fs::copy(&src, &a).expect("copy a.jpg");
    fs::copy(&src, &b).expect("copy b.jpg");

    let pattern = input_dir.path().join("*.jpg");
    let pattern = pattern.to_string_lossy().to_string();

    let mut cmd = imgoptim_cmd();
    let output = cmd
        .args([
            "-m65",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
        ])
        .arg(pattern)
        .output()
        .expect("run imgoptim");
    assert!(output.status.success(), "glob run should succeed");

    expect_jpeg_out(out_dir.path(), "a");
    expect_jpeg_out(out_dir.path(), "b");
}

#[test]
fn output_format_converts_case_insensitive() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/jpeg/photo_nometa.jpg");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--output-format",
        "PNG",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("photo_nometa.png");
    expect_file_exists(&out);
    let bytes = read_bytes(&out);
    assert_is_png(&bytes);
}

#[test]
fn zopfli_options_are_reported() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/png/image_nometa.png");
    assert!(input.exists(), "missing asset: {}", input.display());

    let mut cmd = imgoptim_cmd();
    let output = cmd
        .args([
            "--output-format",
            "png",
            "--png-zopfli",
            "--zopfli-iteration-count",
            "1",
            "--zopfli-max-block-splits",
            "1",
            "--zopfli-timeout",
            "1",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .output()
        .expect("run imgoptim");
    assert!(output.status.success(), "zopfli run should succeed");

    let out = out_dir.path().join("image_nometa.png");
    expect_file_exists(&out);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Zopfli: iteration_count=1 max_block_splits=1 timeout=1s"),
        "stdout should show zopfli status: {stdout}"
    );
}

#[test]
fn dest_flag_after_input_is_parsed() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/jpeg/photo_nometa.jpg");
    assert!(input.exists(), "missing asset: {}", input.display());

    let mut cmd = imgoptim_cmd();
    let output = cmd
        .args([
            input.to_str().unwrap(),
            "--dest",
            out_dir.path().to_str().unwrap(),
            "--overwrite",
        ])
        .output()
        .expect("run imgoptim");
    assert!(output.status.success(), "dest-after-input should succeed");

    expect_jpeg_out(out_dir.path(), "photo_nometa");
}

#[test]
fn keep_metadata_preserves_exif_for_jpeg() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/jpeg/photo_meta.jpg");
    assert!(input.exists(), "missing asset: {}", input.display());

    let input_bytes = read_bytes(&input);
    let has_exif = jpeg_contains_exif(&input_bytes);
    let has_xmp = jpeg_extract_xmp(&input_bytes).is_some();
    assert!(has_exif || has_xmp, "input must contain EXIF or XMP");

    run_ok(&[
        "--keep-metadata",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        input.to_str().unwrap(),
    ]);

    let out = expect_jpeg_out(out_dir.path(), "photo_meta");
    let out_bytes = read_bytes(&out);
    if has_exif {
        assert!(jpeg_contains_exif(&out_bytes), "EXIF should be preserved");
    }
    if has_xmp {
        assert!(
            jpeg_extract_xmp(&out_bytes).is_some(),
            "XMP should be preserved"
        );
    }
}

#[test]
fn overwrite_gain_uses_input_size() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/jpeg/photo_meta.jpg");
    assert!(input.exists(), "missing asset: {}", input.display());

    let preexisting = out_dir.path().join("photo_meta.jpg");
    fs::write(&preexisting, b"x").expect("write preexisting output");

    let mut cmd = imgoptim_cmd();
    let output = cmd
        .args([
            "--quality",
            "10",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .output()
        .expect("run imgoptim");
    assert!(output.status.success(), "overwrite run should succeed");

    let out = expect_jpeg_out(out_dir.path(), "photo_meta");
    let old_bytes = fs::metadata(&input).expect("input metadata").len();
    let new_bytes = fs::metadata(&out).expect("output metadata").len();
    assert_ne!(old_bytes, new_bytes, "expected size change for gain test");

    let expected = if old_bytes == 0 {
        0.0
    } else {
        ((old_bytes.saturating_sub(new_bytes)) as f32) * 100.0 / (old_bytes as f32)
    };
    let expected_str = format!("{expected:.2}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&format!("({expected_str}%)")),
        "stdout should use input size for gain: {stdout}"
    );
}

#[test]
fn quiet_mode_prints_progress() {
    let input_dir = tmp_out_dir();
    let out_dir = tmp_out_dir();
    let src = asset_path("tests/assets/jpeg/photo_nometa.jpg");

    let a = input_dir.path().join("a.jpg");
    let b = input_dir.path().join("b.jpg");
    fs::copy(&src, &a).expect("copy a.jpg");
    fs::copy(&src, &b).expect("copy b.jpg");

    let mut cmd = imgoptim_cmd();
    let output = cmd
        .args([
            "-q",
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            a.to_str().unwrap(),
            b.to_str().unwrap(),
        ])
        .output()
        .expect("run imgoptim");
    assert!(output.status.success(), "quiet run should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Progress: 2/2 (100%)"),
        "stdout should show progress: {stdout}"
    );
}

#[test]
fn verbose_prints_options_and_summary() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/jpeg/photo_nometa.jpg");
    assert!(input.exists(), "missing asset: {}", input.display());

    let mut cmd = imgoptim_cmd();
    let output = cmd
        .args([
            "-v",
            "-m",
            "65",
            "--overwrite",
            "--keep-metadata",
            "--dest",
            out_dir.path().to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .output()
        .expect("run imgoptim");
    assert!(output.status.success(), "verbose run should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&format!(
            "Destination directory: {}",
            out_dir.path().display()
        )),
        "stdout should show destination directory: {stdout}"
    );
    assert!(
        stdout.contains("Image quality limit set to: 65"),
        "stdout should show quality limit: {stdout}"
    );
    assert!(
        stdout.contains("Options: --overwrite --keep-metadata"),
        "stdout should show selected options: {stdout}"
    );
    assert!(
        stdout.contains("[OK]"),
        "stdout should include summary line: {stdout}"
    );
}

#[test]
fn default_summary_is_jpegoptim_like() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/jpeg/photo_nometa.jpg");
    assert!(input.exists(), "missing asset: {}", input.display());

    let mut cmd = imgoptim_cmd();
    let output = cmd
        .args([
            "--overwrite",
            "--dest",
            out_dir.path().to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .output()
        .expect("run imgoptim");
    assert!(output.status.success(), "default run should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[OK]") && stdout.contains("bytes ("),
        "stdout should include jpegoptim-like summary: {stdout}"
    );
}
