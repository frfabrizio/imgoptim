use imgoptim::rules::threshold::{gain_percent, should_replace};

#[test]
fn gain_is_correct() {
    assert!((gain_percent(100, 90) - 10.0).abs() < 1e-6);
    assert!((gain_percent(200, 100) - 50.0).abs() < 1e-6);
    assert!((gain_percent(0, 0) - 0.0).abs() < 1e-6);
}

#[test]
fn should_replace_no_threshold() {
    assert!(should_replace(100, 100, None, false).unwrap());
    assert!(should_replace(100, 110, None, false).unwrap());
}

#[test]
fn should_not_replace_if_no_gain() {
    assert!(!should_replace(100, 100, Some(1.0), false).unwrap());
    assert!(!should_replace(100, 110, Some(1.0), false).unwrap());
}

#[test]
fn should_replace_when_gain_meets_threshold() {
    // 10% gain
    assert!(should_replace(100, 90, Some(10.0), false).unwrap());
    assert!(!should_replace(100, 90, Some(10.1), false).unwrap());
}

#[test]
fn force_overrides_threshold() {
    assert!(should_replace(100, 110, Some(50.0), true).unwrap());
}
