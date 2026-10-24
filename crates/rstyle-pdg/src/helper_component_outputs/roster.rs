//! Complete, canonical typed leaf inventories; absence never means an empty origin.
use super::*;

pub(super) fn leaves(ty: &TypeToken) -> Result<Vec<Output>, String> {
    let ty = match ty {
        TypeToken::Result { ok, .. } if guarded_results::shape(ty) => ok.as_ref(),
        _ => ty,
    };

    if !matches!(ty, TypeToken::Tuple(_) | TypeToken::Struct { .. }) {
        return Err("Component helper result must be a tuple or named product".into());
    }
    let mut outputs = Vec::new();
    visit(ty, &mut Vec::new(), &mut outputs)?;
    Ok(outputs)
}

fn visit(
    ty: &TypeToken,
    selectors: &mut Vec<usize>,
    outputs: &mut Vec<Output>,
) -> Result<(), String> {
    if selectors.len() > crate::concrete_types::MAX_DEPTH || outputs.len() >= 256 {
        return Err("Component output inventory exceeds its explicit budget".into());
    }
    if matches!(ty, TypeToken::Shared(_)) {
        outputs.push(Output {
            selectors: selectors.clone(),
            ty: ty.clone(),
            origins: Vec::new(),
        });
        return Ok(());
    }
    if !crate::data_origins::shared(ty) {
        return Ok(());
    }

    let fields: Vec<_> = match ty {
        TypeToken::Tuple(fields) => fields.iter().collect(),
        TypeToken::Struct { fields, .. } => fields.iter().map(|(_, ty)| ty).collect(),
        _ => return Err("Reference component is outside the tuple/product grammar".into()),
    };
    for (position, field) in fields.into_iter().enumerate() {
        selectors.push(position);
        visit(field, selectors, outputs)?;
        selectors.pop();
    }
    Ok(())
}

pub(super) fn verify(ty: &TypeToken, outputs: &[Output]) -> Result<(), String> {
    let expected = leaves(ty)?;
    if expected.len() != outputs.len() || expected.is_empty() {
        return Err("Incomplete component output inventory".into());
    }
    for (expected, actual) in expected.iter().zip(outputs) {
        if expected.selectors != actual.selectors
            || expected.ty != actual.ty
            || actual.origins.is_empty()
            || !actual.origins.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(
                "Component output selector, type or canonical origin inventory disagrees".into(),
            );
        }
    }
    Ok(())
}

pub(super) fn selected<'a>(
    outputs: &'a [Output],
    selectors: &[usize],
) -> Result<&'a [Output], String> {
    let start = outputs
        .iter()
        .position(|output| output.selectors.starts_with(selectors))
        .ok_or("Selected helper result has no reference component")?;
    let count = outputs[start..]
        .iter()
        .take_while(|output| output.selectors.starts_with(selectors))
        .count();
    Ok(&outputs[start..start + count])
}
