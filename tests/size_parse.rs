use imgoptim::cli::TargetSize;
use imgoptim::rules::size::parse_target_size;

#[test]
fn parse_kb_ok() {
    assert_eq!(
        parse_target_size("120").unwrap(),
        TargetSize::KiloBytes(120)
    );
    assert_eq!(parse_target_size("  1 ").unwrap(), TargetSize::KiloBytes(1));
}

#[test]
fn parse_percent_ok() {
    assert_eq!(parse_target_size("85%").unwrap(), TargetSize::Percent(85));
    assert_eq!(parse_target_size("  1% ").unwrap(), TargetSize::Percent(1));
    assert_eq!(parse_target_size("99%").unwrap(), TargetSize::Percent(99));
}

#[test]
fn parse_percent_reject_0_and_100() {
    assert!(parse_target_size("0%").is_err());
    assert!(parse_target_size("100%").is_err());
}

#[test]
fn parse_kb_reject_0() {
    assert!(parse_target_size("0").is_err());
}

#[test]
fn parse_reject_garbage() {
    assert!(parse_target_size("").is_err());
    assert!(parse_target_size("%").is_err());
    assert!(parse_target_size("abc").is_err());
    assert!(parse_target_size("10%%").is_err());
}
