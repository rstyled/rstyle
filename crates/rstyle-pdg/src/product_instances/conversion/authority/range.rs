//! Reviewed blanket delegation selects an actual local From, never reflexive identity.
use super::*;

pub(in super::super) fn verify(
    canonical: &HirFnFacts,
    observed: &Observation,
    call: &HirNode,
) -> Result<(), String> {
    let registry = reviewed(canonical)?;
    let stable = &observed.into.requested.stable_crate_id;
    selection(registry, &observed.into, INTO, stable)?;

    let from = &observed.from;
    for (definition, key, path) in [
        (&from.requested, FROM[0].0, FROM[0].1),
        (&from.requested_trait, FROM[1].0, FROM[1].1),
        (&from.selected_trait, FROM[2].0, FROM[2].1),
    ] {
        pinned(registry, definition, key, path)?;
        if definition.stable_crate_id != *stable {
            return Err("Range From trait selection changes its observed core identity".into());
        }
    }

    local_definition(canonical, &from.selected)?;
    local_definition(canonical, &from.selected_impl)?;
    if from.selected.path != format!("{}::from", from.selected_impl.path) {
        return Err("Range From method and enclosing implementation paths disagree".into());
    }

    original_call(observed, call)
}

pub(in super::super) fn local_definition(
    canonical: &HirFnFacts,
    definition: &Definition,
) -> Result<(), String> {
    if !local_compiled(
        canonical,
        definition,
        "Missing actual compiled unit for local Range From",
    )? {
        return Err("Selected local From definition differs from its actual compiled unit".into());
    }
    Ok(())
}
