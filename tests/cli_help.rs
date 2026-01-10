use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn help_works() {
    cargo_bin_cmd!("imgoptim").arg("--help").assert().success();
}
