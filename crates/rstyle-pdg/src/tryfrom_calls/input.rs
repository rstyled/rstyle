//! Exact bounded unsigned input evidence shared by source, signatures and calls.
use super::*;

pub(crate) const CAPABILITY: &str = "rust.unsigned-index-conversion-family";

pub(crate) fn captured(facts: &HirFnFacts) -> Result<&str, String> {
    let [input] = facts.signature.params.as_slice() else {
        return Err("Concrete TryFrom requires its single captured unsigned input".into());
    };
    let scalar = input
        .ty
        .normalized
        .as_deref()
        .ok_or("Missing TryFrom input type")?;
    if scalar == "usize"
        || (matches!(scalar, "u16" | "u32" | "u64")
            && crate::extraction_profile::facts_support(facts, CAPABILITY))
    {
        return Ok(scalar);
    }
    Err("TryFrom input lacks its exact registered unsigned contract".into())
}

pub(super) fn structural(identity: &model::Structural, expected: &str) -> Result<(), String> {
    if !matches!(identity, model::Structural::Builtin { name } if name == expected) {
        return Err("TryFrom structural input differs from the exact captured builtin".into());
    }
    Ok(())
}
