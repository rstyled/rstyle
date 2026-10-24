use crate::{
    ParamFact,
    concrete_types::{InterfaceObservations, Observation, TypeTree},
    unit_store::EvidenceFacts,
};

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<&ParamFact, String> {
    if !crate::extraction_profile::facts_support(facts, "rust.consuming-range-conversions") {
        return Err("Owned Range consumption requires its registered capability".into());
    }
    let candidates: Vec<_> = facts
        .signature
        .params
        .iter()
        .filter(|p| p.ty.normalized.as_deref() == Some(super::super::TOKEN))
        .collect();
    let [parameter] = candidates.as_slice() else {
        return Err("Consuming Range requires exactly one whole owned Range formal".into());
    };

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    interface.validate()?;
    let Observation::Captured {
        ty: TypeTree::RangeUsize { observation },
    } = interface
        .parameters
        .get(parameter.position as usize)
        .ok_or("Missing consuming Range interface observation")?
    else {
        return Err("Consuming Range requires its captured owned interface observation".into());
    };

    super::super::authority::verify(observation, facts)?;
    binding(parameter, facts)?;
    Ok(parameter)
}

fn binding(parameter: &ParamFact, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let matches: Vec<_> = facts
        .bindings
        .iter()
        .filter(|binding| binding.hir == parameter.pat.hir)
        .collect();
    let [binding] = matches.as_slice() else {
        return Err("Consuming Range formal binding is not unique".into());
    };
    if !binding.is_param
        || binding.is_mut
        || binding.symbol != parameter.symbol
        || binding.ty.normalized.as_deref() != Some(super::super::TOKEN)
        || parameter.pat.kind != "pat_binding"
        || parameter.pat.binding_mut != Some(false)
        || parameter.pat.binding.as_ref() != Some(&parameter.symbol)
        || !parameter.pat.children.is_empty()
        || parameter.pat.expansion.is_some()
        || !super::super::is_range(&parameter.pat)
    {
        return Err("Consuming Range requires its plain immutable initialized formal".into());
    }

    if facts.bindings.iter().any(|other| {
        other.hir != binding.hir
            && other
                .ty
                .normalized
                .as_deref()
                .is_some_and(|ty| ty.contains("\"kind\":\"range_usize\""))
    }) {
        return Err("Consuming Range does not permit another Range local or alias".into());
    }
    Ok(())
}

pub(super) fn read(node: &crate::HirNode, formal: &ParamFact) -> bool {
    node.kind == "expr_path"
        && node.children.is_empty()
        && node.expansion.is_none()
        && super::super::is_range(node)
        && node.res.as_ref().is_some_and(|resolution| {
            resolution.kind == "local" && resolution.target.as_ref() == Some(&formal.pat.hir)
        })
}
