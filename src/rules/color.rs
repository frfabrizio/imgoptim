use crate::error::ImgOptimError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn parse_hex_rgb(s: &str) -> Result<Rgb8, ImgOptimError> {
    let raw = s.trim();
    let hex = raw.strip_prefix('#').unwrap_or(raw);

    if hex.len() != 6 {
        return Err(ImgOptimError::InvalidArgs(format!(
            "invalid --background (expected #RRGGBB): {raw}"
        )));
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| ImgOptimError::InvalidArgs(format!("invalid --background: {raw}")))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| ImgOptimError::InvalidArgs(format!("invalid --background: {raw}")))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| ImgOptimError::InvalidArgs(format!("invalid --background: {raw}")))?;

    Ok(Rgb8 { r, g, b })
}
