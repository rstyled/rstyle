//! Original diagnostic metadata differs only by independently captured type enrichment.
use super::*;

pub(super) fn verify(original: &Original, local: &HirFnFacts) -> Result<(), String> {
    let mut signature = local.signature.clone();
    crate::source_evidence::exact_pairs(
        signature.params.iter_mut(),
        original.signature.params.iter(),
        "Selected From formal inventories disagree",
        |actual, original| {
            missing_type(&mut actual.ty, &original.ty)?;
            parameter_spelling(actual, original)?;

            missing_node(&mut actual.pat, &original.pat)
        },
    )?;

    missing_type(&mut signature.return_ty, &original.signature.return_ty)?;

    let mut bindings = local.bindings.clone();
    crate::source_evidence::exact_pairs(
        bindings.iter_mut(),
        original.bindings.iter(),
        "Selected From binding inventories disagree",
        |actual, original| missing_type(&mut actual.ty, &original.ty),
    )?;

    let mut body = local.body.clone();
    missing_node(&mut body, &original.body)?;
    if signature != original.signature || bindings != original.bindings || body != original.body {
        return Err(
            "Selected From original HIR/source differs beyond captured normalization".into(),
        );
    }
    Ok(())
}

fn missing_node(actual: &mut HirNode, original: &HirNode) -> Result<(), String> {
    actual.ty = missing_optional_type(&actual.ty, &original.ty)?;
    missing_children(&mut actual.children, &original.children)
}

fn missing_children(actual: &mut [HirNode], original: &[HirNode]) -> Result<(), String> {
    crate::source_evidence::exact_pairs(
        actual.iter_mut(),
        original.iter(),
        "Selected From original node inventories disagree",
        missing_node,
    )
}

fn missing_optional_type(
    actual: &Option<crate::ResolvedTy>,
    original: &Option<crate::ResolvedTy>,
) -> Result<Option<crate::ResolvedTy>, String> {
    match (actual, original) {
        (Some(actual), Some(original)) => {
            let normalized = missing_normalization(actual, original)?;
            Ok(Some(crate::ResolvedTy {
                normalized,
                ..actual.clone()
            }))
        }
        _ => Ok(actual.clone()),
    }
}

fn missing_type(
    actual: &mut crate::ResolvedTy,
    original: &crate::ResolvedTy,
) -> Result<(), String> {
    actual.normalized = missing_normalization(actual, original)?;
    Ok(())
}

fn missing_normalization(
    actual: &crate::ResolvedTy,
    original: &crate::ResolvedTy,
) -> Result<Option<String>, String> {
    if original.normalized.is_none() {
        if let Some(token) = &actual.normalized {
            crate::data_types::parse(token)?;
        }
        return Ok(None);
    }
    Ok(actual.normalized.clone())
}

// Ordinary capture fills the formal pattern spelling from the source type span;
// the diagnostic selection leaves this redundant field empty.
fn parameter_spelling(
    actual: &mut crate::ParamFact,
    original: &crate::ParamFact,
) -> Result<(), String> {
    let (Some(actual_pattern), Some(original_pattern)) = (&mut actual.pat.ty, &original.pat.ty)
    else {
        return Ok(());
    };
    if original_pattern.spelling.is_empty() && !actual_pattern.spelling.is_empty() {
        if actual_pattern.spelling != actual.ty.spelling
            || actual.ty.spelling != original.ty.spelling
        {
            return Err(
                "Selected From parameter spelling differs from its captured source type".into(),
            );
        }
        actual_pattern.spelling.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests;
