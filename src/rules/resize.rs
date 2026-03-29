use crate::error::ImgOptimError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeSpec {
    pub w: Option<u32>,
    pub h: Option<u32>,
}

/// Parse `1920x1080`, `1920x`, `x1080`.
///
/// # Errors
///
/// Returns an error when the resize spec is empty, malformed, or contains
/// invalid dimensions.
pub fn parse_resize_spec(s: &str) -> Result<ResizeSpec, ImgOptimError> {
    let raw = s.trim();
    if raw.is_empty() {
        return Err(ImgOptimError::InvalidArgs("--resize is empty".into()));
    }

    let (width_part, height_part) = raw.split_once('x').ok_or_else(|| {
        ImgOptimError::InvalidArgs(format!("invalid --resize: {raw} (expected WxH, Wx, xH)"))
    })?;

    let width =
        if width_part.trim().is_empty() {
            None
        } else {
            Some(width_part.trim().parse::<u32>().map_err(|_| {
                ImgOptimError::InvalidArgs(format!("invalid --resize width: {raw}"))
            })?)
        };
    let height =
        if height_part.trim().is_empty() {
            None
        } else {
            Some(height_part.trim().parse::<u32>().map_err(|_| {
                ImgOptimError::InvalidArgs(format!("invalid --resize height: {raw}"))
            })?)
        };

    if width.is_none() && height.is_none() {
        return Err(ImgOptimError::InvalidArgs(format!(
            "invalid --resize: {raw} (missing both width and height)"
        )));
    }
    if let Some(0) = width {
        return Err(ImgOptimError::InvalidArgs(
            "--resize width must be > 0".into(),
        ));
    }
    if let Some(0) = height {
        return Err(ImgOptimError::InvalidArgs(
            "--resize height must be > 0".into(),
        ));
    }

    Ok(ResizeSpec {
        w: width,
        h: height,
    })
}
