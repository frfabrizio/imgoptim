use imgoptim::rules::resize::{parse_resize_spec, ResizeSpec};

#[test]
fn parse_resize_both() {
    assert_eq!(
        parse_resize_spec("1920x1080").unwrap(),
        ResizeSpec {
            w: Some(1920),
            h: Some(1080),
        }
    );
}

#[test]
fn parse_resize_width_only() {
    assert_eq!(
        parse_resize_spec("1920x").unwrap(),
        ResizeSpec {
            w: Some(1920),
            h: None
        }
    );
}

#[test]
fn parse_resize_height_only() {
    assert_eq!(
        parse_resize_spec("x1080").unwrap(),
        ResizeSpec {
            w: None,
            h: Some(1080)
        }
    );
}

#[test]
fn parse_resize_trims() {
    assert_eq!(
        parse_resize_spec("  800x600 ").unwrap(),
        ResizeSpec {
            w: Some(800),
            h: Some(600)
        }
    );
}

#[test]
fn parse_resize_rejects_invalid() {
    assert!(parse_resize_spec("").is_err());
    assert!(parse_resize_spec("800").is_err());
    assert!(parse_resize_spec("x").is_err());
    assert!(parse_resize_spec("0x600").is_err());
    assert!(parse_resize_spec("800x0").is_err());
    assert!(parse_resize_spec("axb").is_err());
}
