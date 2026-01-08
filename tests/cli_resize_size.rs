mod common;

use common::*;

use png::Decoder;

#[test]
fn convert_resize_updates_dimensions() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/png/image_nometa.png");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--name-suffix",
        "_small",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        "convert",
        "--output",
        "png",
        "--resize",
        "32x",
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("image_nometa_small.png");
    expect_file_exists(&out);

    let bytes = read_bytes(&out);
    let decoder = Decoder::new(std::io::Cursor::new(bytes));
    let reader = decoder.read_info().expect("png read_info");
    let info = reader.info();
    assert_eq!(info.width, 32);
}

#[test]
fn convert_size_target_not_reached_skips_write() {
    let out_dir = tmp_out_dir();
    let input = asset_path("tests/assets/png/image_meta.png");
    assert!(input.exists(), "missing asset: {}", input.display());

    run_ok(&[
        "--size",
        "1%",
        "--name-suffix",
        "_tiny",
        "--overwrite",
        "--dest",
        out_dir.path().to_str().unwrap(),
        "convert",
        "--output",
        "jpeg",
        "--background",
        "#ffffff",
        input.to_str().unwrap(),
    ]);

    let out = out_dir.path().join("image_meta_tiny.jpg");
    assert!(!out.exists(), "output should not be created when target is unreachable");
}
