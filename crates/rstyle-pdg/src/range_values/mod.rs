//! Fresh non-Copy Range results, without local storage or general move semantics.
pub mod authority;
pub(crate) mod consuming;
mod selection;

use crate::{HirNode, concrete_types::Observation, unit_store::EvidenceFacts};

pub const CAPABILITY: &str = "rust.fresh-range-results";
pub(crate) const CONSUMING_CAPABILITY: &str = "rust.consuming-range-conversions";
pub(crate) const TOKEN: &str = r#"{"kind":"range_usize"}"#;

pub(crate) fn is_range(node: &HirNode) -> bool {
    node.ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        .is_some_and(|text| text == TOKEN)
}

pub(crate) fn used(facts: &dyn EvidenceFacts) -> bool {
    facts
        .signature
        .params
        .iter()
        .map(|p| &p.ty)
        .chain(facts.bindings.iter().map(|b| &b.ty))
        .any(|ty| {
            ty.normalized
                .as_deref()
                .is_some_and(|text| text.contains("\"kind\":\"range_usize\""))
        })
        || facts
            .signature
            .return_ty
            .normalized
            .as_deref()
            .is_some_and(|text| text.contains("\"kind\":\"range_usize\""))
        || facts.body.any(&is_range)
}

pub(crate) fn verify_function(facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !used(facts) {
        return Ok(());
    }
    if consuming_input(facts) {
        if !crate::extraction_profile::facts_support(facts, CONSUMING_CAPABILITY) {
            return Err("Consuming Range input requires its registered capability".into());
        }
        return consuming::verify_fields(facts).map(|_| ());
    }
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Fresh Range results require their registered capability".into());
    }

    let interface = facts
        .build
        .frontend_options
        .data::<crate::concrete_types::InterfaceObservations>()?;
    let Observation::Captured {
        ty: crate::concrete_types::TypeTree::RangeUsize { observation },
    } = &interface.result
    else {
        return Err("Range values require an exact fresh function result".into());
    };

    authority::verify(observation, facts)?;
    for ty in facts
        .signature
        .params
        .iter()
        .map(|p| &p.ty)
        .chain(facts.bindings.iter().map(|b| &b.ty))
    {
        if ty
            .normalized
            .as_deref()
            .is_some_and(|s| s.contains("\"kind\":\"range_usize\""))
        {
            return Err("Range parameters and local storage require move/liveness proof".into());
        }
    }

    let source: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    crate::source_evidence::closed_body(&source.block)?;
    fresh(&facts.body, true)
}

pub(crate) fn consuming_input(facts: &dyn EvidenceFacts) -> bool {
    facts
        .signature
        .params
        .iter()
        .any(|parameter| parameter.ty.normalized.as_deref() == Some(TOKEN))
}

pub(crate) fn value_field(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<usize>, String> {
    if node.kind != "expr_field"
        || !node.children.first().is_some_and(is_range)
        || !crate::extraction_profile::facts_support(facts, CONSUMING_CAPABILITY)
    {
        return Ok(None);
    }
    consuming::verify_fields(facts)?.position(node)
}

fn fresh(node: &HirNode, returning: bool) -> Result<(), String> {
    let range = is_range(node);
    if range
        && (!(returning || node.kind == "expr_ret")
            || !matches!(
                node.kind.as_str(),
                "expr_struct" | "expr_call" | "expr_method_call" | "expr_block" | "expr_ret"
            ))
    {
        return Err("Range value is not a fresh constructor/helper result forwarded whole".into());
    }
    for (position, child) in node.children.iter().enumerate() {
        let tail = node.kind == "expr_block" && returning && position + 1 == node.children.len();
        fresh(child, tail || node.kind == "expr_ret")?;
    }
    Ok(())
}

pub(crate) use selection::verify as verify_selection;

/// Fresh Range results can carry verified reads without claiming Copy or panic cleanup.
pub(crate) fn read_result(facts: &dyn EvidenceFacts, pair: crate::producer::PairView<'_>) -> bool {
    facts.signature.return_ty.normalized.as_deref() == Some(TOKEN)
        && pair.cfg.extraction_evidence.status == crate::ExtractionStatus::Complete
        && verify_function(facts).is_ok()
}

/// Only the result changes; the receiver still needs the existing exact Copy proof.
pub(crate) fn receiver_result(
    observed: &Observation,
    facts: &dyn EvidenceFacts,
) -> Result<bool, String> {
    let Observation::Captured {
        ty: crate::concrete_types::TypeTree::RangeUsize { observation },
    } = observed
    else {
        return Ok(false);
    };
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Ok(false);
    }

    authority::verify(observation, facts)?;
    verify_function(facts)?;

    Ok(true)
}
