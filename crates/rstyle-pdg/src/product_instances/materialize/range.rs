//! Retain real local conversion evidence; Range is never an identity summary.
use super::*;

pub fn verify_range_constructor_projection(
    canonical: &HirFnFacts,
    observation: &Value,
    local: &HirFnFacts,
) -> Result<CheckedProjection, String> {
    let prepared = Prepared::new(canonical, observation)?;
    let conversion = conversion::verify_range(canonical, observation, prepared.span()?, local)?;

    let mut checked = prepared.project(&conversion, SpanArgument::RangeUsize)?;
    let transfer = crate::range_values::consuming::verify_transfer(
        &checked.facts,
        &checked.operation,
        canonical,
    )?;

    if transfer.formal_position() != 1 || transfer.receiver() != checked.receiver() {
        return Err(
            "Range conversion and affine formal proof disagree on the exact saved input".into(),
        );
    }

    checked.local_from = Some(Box::new(local.clone()));
    Ok(checked)
}

#[cfg(test)]
mod tests;
