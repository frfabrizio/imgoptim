use imgoptim::rules::color::{ parse_hex_rgb, Rgb8 };

#[test]
fn parse_hex_rgb_ok() {
    assert_eq!(parse_hex_rgb("#ffffff").unwrap(), Rgb8 { r: 255, g: 255, b: 255 });
    assert_eq!(parse_hex_rgb("#000000").unwrap(), Rgb8 { r: 0, g: 0, b: 0 });
    assert_eq!(parse_hex_rgb("#12aBcF").unwrap(), Rgb8 { r: 0x12, g: 0xab, b: 0xcf });
}

#[test]
fn parse_hex_rgb_accepts_without_hash() {
    assert_eq!(parse_hex_rgb("ff0000").unwrap(), Rgb8 { r: 255, g: 0, b: 0 });
}

#[test]
fn parse_hex_rgb_rejects_invalid() {
    assert!(parse_hex_rgb("").is_err());
    assert!(parse_hex_rgb("#fff").is_err());
    assert!(parse_hex_rgb("#fffffff").is_err());
    assert!(parse_hex_rgb("#gg0000").is_err());
}
