//! Checked work-budget accounting shared by restoration and optional exports.
pub(crate) fn charge(remaining: &mut usize, bytes: usize, reason: &str) -> Result<(), String> {
    *remaining = remaining.checked_sub(bytes).ok_or(reason)?;
    Ok(())
}
