mod common;

use common::*;

#[test]
fn convert_only_filter_skips_non_matching_input() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/png/image_nometa.png");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--only",
        "jpeg",
        "--name-suffix",
        "_imgoptim",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        "convert",
        "--output",
        "jpeg",
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("image_nometa_imgoptim.jpg");
    assert!(!out.exists(), "output should not be created when --only skips input");
}

#[test]
fn convert_skip_filter_skips_matching_input() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/png/image_nometa.png");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--skip",
        "png",
        "--name-suffix",
        "_imgoptim",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        "convert",
        "--output",
        "jpeg",
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("image_nometa_imgoptim.jpg");
    assert!(!out.exists(), "output should not be created when --skip excludes input");
}

#[test]
fn convert_input_filter_skips_other_formats() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/jpeg/photo_meta.jpg");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--name-suffix",
        "_imgoptim",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        "convert",
        "--output",
        "png",
        "--input",
        "png",
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("photo_meta_imgoptim.png");
    assert!(!out.exists(), "output should not be created when --input does not match");
}

#[test]
fn optimize_name_suffix_and_dest_write_to_target_dir() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/png/image_nometa.png");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--name-suffix",
        "_imgoptim",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("image_nometa_imgoptim.png");
    expect_file_exists(&out);
    let bytes = read_bytes(&out);
    assert_is_png(&bytes);
}

#[test]
fn convert_keep_ext_uses_default_conv_suffix() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/jpeg/photo_nometa.jpg");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--keep-ext",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        "convert",
        "--output",
        "png",
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("photo_nometa.conv.jpg");
    expect_file_exists(&out);
    let bytes = read_bytes(&out);
    assert_is_png(&bytes);
}

#[test]
fn convert_keep_ext_with_custom_suffix() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/png/image_nometa.png");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--keep-ext",
        "--name-suffix",
        "_imgoptim",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        "convert",
        "--output",
        "jpeg",
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("image_nometa_imgoptim.png");
    expect_file_exists(&out);
    let bytes = read_bytes(&out);
    assert_is_jpeg(&bytes);
}

#[test]
fn optimize_only_filter_skips_non_matching_input() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/png/image_nometa.png");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--only",
        "jpeg",
        "--name-suffix",
        "_imgoptim",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("image_nometa_imgoptim.png");
    assert!(!out.exists(), "output should not be created when --only skips input");
}

#[test]
fn optimize_skip_filter_skips_matching_input() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/png/image_nometa.png");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--skip",
        "png",
        "--name-suffix",
        "_imgoptim",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("image_nometa_imgoptim.png");
    assert!(!out.exists(), "output should not be created when --skip excludes input");
}

#[test]
fn convert_inplace_replaces_extension() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/jpeg/photo_nometa.jpg");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--inplace",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        "convert",
        "--output",
        "png",
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("photo_nometa.png");
    expect_file_exists(&out);
    let bytes = read_bytes(&out);
    assert_is_png(&bytes);
}
