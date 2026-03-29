mod common;

use common::*;

use filetime::{set_file_mtime, FileTime};
use png::{BitDepth, ColorType, Encoder};
use std::fs;

#[test]
fn convert_background_affects_output_bytes() {
    let out_dir = tmp_out_dir();
    let input_dir = tmp_out_dir();
    let input = input_dir.path().join("alpha.png");

    // Create a 1x1 transparent PNG so background impacts JPEG output.
    let pixel = [10u8, 20u8, 30u8, 0u8];
    let mut data = Vec::new();
    {
        let mut enc = Encoder::new(&mut data, 1, 1);
        enc.set_color(ColorType::Rgba);
        enc.set_depth(BitDepth::Eight);
        let mut writer = enc.write_header().expect("png write_header");
        writer
            .write_image_data(&pixel)
            .expect("png write_image_data");
    }
    std::fs::write(&input, &data).expect("write temp png");

    run_ok(&[
        "--name-suffix",
        "_white",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        "--output-format",
        "jpeg",
        "--background",
        "#ffffff",
        input.to_str().unwrap(),
    ]);

    run_ok(&[
        "--name-suffix",
        "_black",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        "--output-format",
        "jpeg",
        "--background",
        "#000000",
        input.to_str().unwrap(),
    ]);

    let out_white = out_dir.path().join("alpha_white.jpg");
    let out_black = out_dir.path().join("alpha_black.jpg");
    expect_file_exists(&out_white);
    expect_file_exists(&out_black);

    let white_bytes = read_bytes(&out_white);
    let black_bytes = read_bytes(&out_black);
    assert_is_jpeg(&white_bytes);
    assert_is_jpeg(&black_bytes);
    assert!(
        white_bytes != black_bytes,
        "expected different outputs for different backgrounds"
    );
}

#[test]
fn noaction_does_not_write_output() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/png/image_nometa.png");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--noaction",
        "--name-suffix",
        "_imgoptim",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("image_nometa_imgoptim.png");
    assert!(
        !out.exists(),
        "output should not be created in noaction mode"
    );
}

#[test]
fn preserve_copies_timestamps() {
    let out_dir = tmp_out_dir();
    let input_src = asset_path("tests/assets/png/image_nometa.png");
    assert!(input_src.exists(), "missing asset: {}", input_src.display());

    let input = out_dir.path().join("image_nometa.png");
    fs::copy(&input_src, &input).expect("copy test asset");

    let ts = FileTime::from_unix_time(946_684_800, 0);
    set_file_mtime(&input, ts).expect("set mtime");

    run_ok(&[
        "--preserve",
        "--name-suffix",
        "_imgoptim",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("image_nometa_imgoptim.png");
    expect_file_exists(&out);

    let out_meta = fs::metadata(&out).expect("metadata");
    let out_mtime = FileTime::from_last_modification_time(&out_meta);
    assert_eq!(out_mtime, ts, "mtime should be preserved");
}

#[test]
fn threshold_prevents_write_but_force_overrides() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/png/image_nometa.png");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--threshold",
        "100",
        "--name-suffix",
        "_thr",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("image_nometa_thr.png");
    assert!(
        !out.exists(),
        "output should not be created when threshold is too high"
    );

    run_ok(&[
        "--threshold",
        "100",
        "--force",
        "--name-suffix",
        "_thr_force",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        input.to_str().unwrap(),
    ]);

    let out_force = out_dir.path().join("image_nometa_thr_force.png");
    expect_file_exists(&out_force);
}

#[test]
fn dest_path_with_extension_is_treated_as_directory_yes_creates() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/jpeg/photo_nometa.jpg");
    assert!(input.exists(), "missing asset: {}", input.display());

    let dest = out_dir
        .path()
        .join("Mont Blanc")
        .join("Optim")
        .join("test.jpg");

    run_ok_with_input(
        &[
            "-m65",
            "--overwrite",
            "--dest",
            dest.to_str().unwrap(),
            input.to_str().unwrap(),
        ],
        "o\n",
    );

    let out = expect_jpeg_out(&dest, "photo_nometa");
    let bytes = read_bytes(&out);
    assert_is_jpeg(&bytes);
}

#[test]
fn dest_path_with_extension_is_treated_as_directory_no_aborts() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/jpeg/photo_nometa.jpg");
    assert!(input.exists(), "missing asset: {}", input.display());

    let dest = out_dir
        .path()
        .join("Mont Blanc")
        .join("Optim")
        .join("test_no.jpg");

    let mut cmd = imgoptim_cmd();
    cmd.args([
        "-m65",
        "--overwrite",
        "--dest",
        dest.to_str().unwrap(),
        input.to_str().unwrap(),
    ])
    .write_stdin("n\n")
    .assert()
    .failure();

    assert!(
        !dest.exists(),
        "destination directory should not be created"
    );
}
