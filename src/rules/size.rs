use crate::cli::TargetSize;
use crate::error::ImgOptimError;

/// Parse --size value:
/// - "120"  => 120 KB (1..n)
/// - "85%"  => 85 %  (1..99)
pub fn parse_target_size(s: &str) -> Result<TargetSize, ImgOptimError> {
    let raw = s.trim();
    if raw.is_empty() {
        return Err(ImgOptimError::InvalidArgs("--size is empty".into()));
    }

    if let Some(num) = raw.strip_suffix('%') {
        let num = num.trim();
        if num.is_empty() {
            return Err(ImgOptimError::InvalidArgs(
                "--size percent is missing".into(),
            ));
        }
        let p: u8 = num
            .parse()
            .map_err(|_| ImgOptimError::InvalidArgs(format!("--size invalid percent: {raw}")))?;
        if !(1..=99).contains(&p) {
            return Err(ImgOptimError::InvalidArgs(
                "--size percent must be 1%..99%".into(),
            ));
        }
        return Ok(TargetSize::Percent(p));
    }

    // KB form
    let kb: u64 = raw
        .parse()
        .map_err(|_| ImgOptimError::InvalidArgs(format!("--size invalid KB value: {raw}")))?;
    if kb < 1 {
        return Err(ImgOptimError::InvalidArgs("--size KB must be >= 1".into()));
    }
    Ok(TargetSize::KiloBytes(kb))
}
