use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn help_works() {
    let output = cargo_bin_cmd!("imgoptim")
        .arg("--help")
        .output()
        .expect("run --help");
    assert!(output.status.success(), "help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("--webp-quality"),
        "help should not mention --webp-quality"
    );
    assert!(
        !stdout.contains("--webp-method"),
        "help should not mention --webp-method"
    );
    assert!(
        stdout.contains("--jpeg-normal"),
        "help should mention --jpeg-normal"
    );
    assert!(
        stdout.contains("--jpeg-progressive"),
        "help should mention --jpeg-progressive"
    );
    assert!(
        stdout.contains("--jpeg-sampling"),
        "help should mention --jpeg-sampling"
    );
    assert!(
        stdout.contains("--png-zopfli"),
        "help should mention --png-zopfli"
    );
    assert!(
        stdout.contains("--output-format"),
        "help should mention --output-format"
    );
    assert!(
        stdout.contains("--zopfli-iteration-count"),
        "help should mention --zopfli-iteration-count"
    );
    assert!(
        stdout.contains("--zopfli-max-block-splits"),
        "help should mention --zopfli-max-block-splits"
    );
    assert!(
        stdout.contains("--zopfli-timeout"),
        "help should mention --zopfli-timeout"
    );
}
