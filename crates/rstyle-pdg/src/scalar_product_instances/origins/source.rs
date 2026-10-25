use super::*;
use crate::slice_patterns::semantics::Region;

pub(crate) fn source(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Option<Vec<Origin>>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    let checked = activation::verify(facts)?;

    if let Some(origins) = direct(node, &checked)? {
        if !selectors.is_empty() {
            return Err("Direct prefix references cannot select aggregate components".into());
        }
        return Ok(Some(origins));
    }
    if node == checked.constructor().node() {
        return Ok(Some(component(selectors, &checked)?));
    }
    if node == checked.some() && selectors.first() == Some(&0) {
        return Ok(Some(component(&selectors[1..], &checked)?));
    }
    if node == checked.prefix().failure() && selectors.is_empty() {
        return Ok(Some(Vec::new()));
    }
    if aggregate(node, &checked) {
        return Err(
            "Guarded digit product requires its complete tagged component inventory".into(),
        );
    }

    Ok(None)
}

fn component(selectors: &[usize], checked: &Checked<'_>) -> Result<Vec<Origin>, String> {
    match selectors {
        [0] => Ok(vec![suffix(checked)?]),
        [1] => Ok(Vec::new()),
        _ => Err("Digit product requires one exact declared component selector".into()),
    }
}

pub(super) fn suffix(checked: &Checked<'_>) -> Result<Origin, String> {
    Region::Suffix {
        start: checked.prefix().consumed(),
    }
    .checked_origin(checked.prefix())
}

fn direct(node: &HirNode, checked: &Checked<'_>) -> Result<Option<Vec<Origin>>, String> {
    let prefix = checked.prefix();
    if node == prefix.formal() || node == prefix.scrutinee() {
        return Ok(Some(vec![input()]));
    }
    for (position, digit) in prefix.digits().iter().enumerate() {
        if node == digit.binding() || node == digit.pointer() {
            return Ok(Some(vec![
                Region::Element {
                    position: position as u8,
                }
                .checked_origin(prefix)?,
            ]));
        }
    }
    if node == prefix.prefix().remainder().binding() || node == prefix.remainder() {
        return Ok(Some(vec![suffix(checked)?]));
    }

    Ok(None)
}

pub(super) fn input() -> Origin {
    Origin {
        parameter: 0,
        path: vec![crate::field_locations::Projection::SharedReferent],
    }
}

fn aggregate(node: &HirNode, checked: &Checked<'_>) -> bool {
    [
        checked.root(),
        checked.prefix().matched(),
        checked.prefix().success(),
        checked.prefix().failure(),
        checked.some(),
        checked.constructor().node(),
    ]
    .contains(&node)
}

pub(crate) fn verify_source_value(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let checked = activation::verify(facts)?;
    if !aggregate(node, &checked) {
        return Err(
            "Reference-bearing digit value is outside its complete guarded source inventory".into(),
        );
    }
    // Field zero is the saved suffix, field one the exact concrete scalar.
    // Option cases stay distinct, including the explicitly reference-free None.
    checked.instance().check_shape()?;
    suffix(&checked)?;

    Ok(())
}
