use crate::error::ImgOptimError;

/// Compute gain percentage: 100 * (old - new) / old
#[must_use]
pub fn gain_percent(old_bytes: u64, new_bytes: u64) -> f32 {
    if old_bytes == 0 {
        return 0.0;
    }
    let diff = old_bytes.saturating_sub(new_bytes) as f64;
    ((diff * 100.0) / (old_bytes as f64)) as f32
}

/// Decide if we should replace target based on threshold and force.
/// - If threshold is None => replace.
/// - If force => replace.
/// - If new >= old => no gain => replace only if force (handled above), otherwise keep.
/// - Else replace if gain >= threshold.
pub fn should_replace(
    old_bytes: u64,
    new_bytes: u64,
    threshold_percent: Option<f32>,
    force: bool,
) -> Result<bool, ImgOptimError> {
    if force {
        return Ok(true);
    }
    let Some(th) = threshold_percent else {
        return Ok(true);
    };
    if !(0.0..=100.0).contains(&th) {
        return Err(ImgOptimError::InvalidArgs(
            "--threshold must be 0..100".into(),
        ));
    }

    // No improvement => do not replace
    if new_bytes >= old_bytes {
        return Ok(false);
    }

    let gain = gain_percent(old_bytes, new_bytes);
    Ok(gain >= th)
}
