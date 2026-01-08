use crate::cli::FitMode;

fn clamp_u32(v: i64, lo: i64, hi: i64) -> u32 {
    v.max(lo).min(hi) as u32
}

fn compute_dims(
    src_w: u32,
    src_h: u32,
    req_w: Option<u32>,
    req_h: Option<u32>,
    fit: FitMode,
) -> (u32, u32) {
    // If one side missing, preserve aspect ratio.
    let (mut tw, mut th) = match (req_w, req_h) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            let h = (((src_h as f64) * (w as f64)) / (src_w as f64))
                .round()
                .max(1.0) as u32;
            (w, h)
        }
        (None, Some(h)) => {
            let w = (((src_w as f64) * (h as f64)) / (src_h as f64))
                .round()
                .max(1.0) as u32;
            (w, h)
        }
        (None, None) => (src_w, src_h),
    };

    // For contain/cover with both specified: adjust to preserve ratio.
    if req_w.is_some() && req_h.is_some() {
        let src_ar = (src_w as f64) / (src_h as f64);
        let dst_ar = (tw as f64) / (th as f64);

        match fit {
            FitMode::Stretch => { /* keep tw/th */ }
            FitMode::Contain => {
                // fit inside box
                if dst_ar > src_ar {
                    tw = ((th as f64) * src_ar).round().max(1.0) as u32;
                } else {
                    th = ((tw as f64) / src_ar).round().max(1.0) as u32;
                }
            }
            FitMode::Cover => {
                // cover box (crop later)
                if dst_ar > src_ar {
                    th = ((tw as f64) / src_ar).round().max(1.0) as u32;
                } else {
                    tw = ((th as f64) * src_ar).round().max(1.0) as u32;
                }
            }
        }
    }

    (tw.max(1), th.max(1))
}

fn bilinear_sample_u8(c00: u8, c10: u8, c01: u8, c11: u8, fx: f32, fy: f32) -> u8 {
    let a = (c00 as f32) + ((c10 as f32) - (c00 as f32)) * fx;
    let b = (c01 as f32) + ((c11 as f32) - (c01 as f32)) * fx;
    let v = a + (b - a) * fy;
    v.round().clamp(0.0, 255.0) as u8
}

pub fn resize_rgb_bilinear(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    req_w: Option<u32>,
    req_h: Option<u32>,
    fit: FitMode,
) -> (u32, u32, Vec<u8>) {
    let (tw, th) = compute_dims(src_w, src_h, req_w, req_h, fit);

    if tw == src_w && th == src_h {
        return (src_w, src_h, src.to_vec());
    }

    let mut out = vec![0u8; (tw * th * 3) as usize];

    let sx = (src_w as f32) / (tw as f32);
    let sy = (src_h as f32) / (th as f32);

    for y in 0..th {
        let fy = ((y as f32) + 0.5) * sy - 0.5;
        let y0 = clamp_u32(fy.floor() as i64, 0, (src_h as i64) - 1);
        let y1 = clamp_u32((y0 as i64) + 1, 0, (src_h as i64) - 1);
        let wy = (fy - fy.floor()).clamp(0.0, 1.0);

        for x in 0..tw {
            let fx = ((x as f32) + 0.5) * sx - 0.5;
            let x0 = clamp_u32(fx.floor() as i64, 0, (src_w as i64) - 1);
            let x1 = clamp_u32((x0 as i64) + 1, 0, (src_w as i64) - 1);
            let wx = (fx - fx.floor()).clamp(0.0, 1.0);

            let idx00 = ((y0 * src_w + x0) * 3) as usize;
            let idx10 = ((y0 * src_w + x1) * 3) as usize;
            let idx01 = ((y1 * src_w + x0) * 3) as usize;
            let idx11 = ((y1 * src_w + x1) * 3) as usize;

            let o = ((y * tw + x) * 3) as usize;

            for c in 0..3 {
                out[o + c] = bilinear_sample_u8(
                    src[idx00 + c],
                    src[idx10 + c],
                    src[idx01 + c],
                    src[idx11 + c],
                    wx,
                    wy,
                );
            }
        }
    }

    (tw, th, out)
}

pub fn resize_rgba_bilinear(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    req_w: Option<u32>,
    req_h: Option<u32>,
    fit: FitMode,
) -> (u32, u32, Vec<u8>) {
    let (tw, th) = compute_dims(src_w, src_h, req_w, req_h, fit);

    if tw == src_w && th == src_h {
        return (src_w, src_h, src.to_vec());
    }

    let mut out = vec![0u8; (tw * th * 4) as usize];

    let sx = (src_w as f32) / (tw as f32);
    let sy = (src_h as f32) / (th as f32);

    for y in 0..th {
        let fy = ((y as f32) + 0.5) * sy - 0.5;
        let y0 = clamp_u32(fy.floor() as i64, 0, (src_h as i64) - 1);
        let y1 = clamp_u32((y0 as i64) + 1, 0, (src_h as i64) - 1);
        let wy = (fy - fy.floor()).clamp(0.0, 1.0);

        for x in 0..tw {
            let fx = ((x as f32) + 0.5) * sx - 0.5;
            let x0 = clamp_u32(fx.floor() as i64, 0, (src_w as i64) - 1);
            let x1 = clamp_u32((x0 as i64) + 1, 0, (src_w as i64) - 1);
            let wx = (fx - fx.floor()).clamp(0.0, 1.0);

            let idx00 = ((y0 * src_w + x0) * 4) as usize;
            let idx10 = ((y0 * src_w + x1) * 4) as usize;
            let idx01 = ((y1 * src_w + x0) * 4) as usize;
            let idx11 = ((y1 * src_w + x1) * 4) as usize;

            let o = ((y * tw + x) * 4) as usize;

            for c in 0..4 {
                out[o + c] = bilinear_sample_u8(
                    src[idx00 + c],
                    src[idx10 + c],
                    src[idx01 + c],
                    src[idx11 + c],
                    wx,
                    wy,
                );
            }
        }
    }

    (tw, th, out)
}
