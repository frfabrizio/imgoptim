use crate::error::ImgOptimError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeSpec {
    pub w: Option<u32>,
    pub h: Option<u32>,
}

/// Parse "1920x1080", "1920x", "x1080"
pub fn parse_resize_spec(s: &str) -> Result<ResizeSpec, ImgOptimError> {
    let raw = s.trim();
    if raw.is_empty() {
        return Err(ImgOptimError::InvalidArgs("--resize is empty".into()));
    }

    let (a, b) = raw.split_once('x').ok_or_else(|| {
        ImgOptimError::InvalidArgs(format!("invalid --resize: {raw} (expected WxH, Wx, xH)"))
    })?;

    let w =
        if a.trim().is_empty() {
            None
        } else {
            Some(a.trim().parse::<u32>().map_err(|_| {
                ImgOptimError::InvalidArgs(format!("invalid --resize width: {raw}"))
            })?)
        };
    let h =
        if b.trim().is_empty() {
            None
        } else {
            Some(b.trim().parse::<u32>().map_err(|_| {
                ImgOptimError::InvalidArgs(format!("invalid --resize height: {raw}"))
            })?)
        };

    if w.is_none() && h.is_none() {
        return Err(ImgOptimError::InvalidArgs(format!(
            "invalid --resize: {raw} (missing both width and height)"
        )));
    }
    if let Some(0) = w {
        return Err(ImgOptimError::InvalidArgs(
            "--resize width must be > 0".into(),
        ));
    }
    if let Some(0) = h {
        return Err(ImgOptimError::InvalidArgs(
            "--resize height must be > 0".into(),
        ));
    }

    Ok(ResizeSpec { w, h })
}
