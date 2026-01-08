use imgoptim::cli::FitMode;
use imgoptim::formats::resize::resize_rgb_bilinear;

#[test]
fn resize_identity_returns_same() {
    let src_w = 4;
    let src_h = 3;
    let src = vec![10u8; (src_w * src_h * 3) as usize];

    let (w, h, out) = resize_rgb_bilinear(&src, src_w, src_h, Some(4), Some(3), FitMode::Stretch);
    assert_eq!(w, 4);
    assert_eq!(h, 3);
    assert_eq!(out.len(), src.len());
}

#[test]
fn resize_width_only_preserves_aspect() {
    let src_w = 400;
    let src_h = 200;
    let src = vec![0u8; (src_w * src_h * 3) as usize];

    let (w, h, out) = resize_rgb_bilinear(&src, src_w, src_h, Some(200), None, FitMode::Contain);
    assert_eq!(w, 200);
    assert_eq!(h, 100);
    assert_eq!(out.len(), (200 * 100 * 3) as usize);
}
