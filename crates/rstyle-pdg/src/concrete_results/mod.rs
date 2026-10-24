//! Fresh concrete Results with separately proved no-drop or Copy errors.
//!
//! Whole construction/forwarding composes with independently checked propagation.
//! Copy errors retain their enum vocabulary; local Result storage stays closed.
mod constructors;
pub(crate) mod copy_errors;
pub(crate) mod documents;
pub(crate) mod fresh_products;
mod interface;
pub(crate) mod scalar_errors;
pub(crate) mod source;
mod types;

use crate::{HirNode, concrete_types::TypeTree, data_types::TypeToken, unit_store::EvidenceFacts};

pub const CAPABILITY: &str = "rust.fresh-concrete-results";
pub const RESOLUTION: &str = "resolved_result_variant";
pub(crate) const KEY: &str = "rstyle.data:concrete-result";
pub(crate) const SOURCE_DIGEST: &str =
    "sha256:777e0f516deba3899c8f16c5511da3ebe7debe23f4bcf6a6353c98a927c0f7d1";

pub use constructors::{Constructor, verify_constructor};
pub(crate) use constructors::{constructor_candidate, constructors};
pub(crate) use types::payload;

/// Verify the exact fresh return boundary, including non-Copy error constructors.
pub(crate) fn verify_function(facts: &dyn EvidenceFacts) -> Result<(), String> {
    if transport_interface(facts)? {
        return verify_transport(facts);
    }
    if fresh_products::interface_used(facts)? {
        return fresh_products::verify_product_returns(facts);
    }
    if !types::used(facts)? {
        return Ok(());
    }

    let interface = facts
        .build
        .frontend_options
        .data::<crate::concrete_types::InterfaceObservations>()?;
    let crate::concrete_types::Observation::Captured { ty } = &interface.result else {
        return Err("Fresh Result/error requires complete captured return evidence".into());
    };

    let error = types::return_error(ty)
        .ok_or("Fresh Result/error values require a whole supported function result")?;
    types::checked_error(error, facts)?;
    let scalar_boundary: Option<scalar_errors::Boundary> = match TypeToken::from_tree(ty) {
        TypeToken::Result { ok, err } if scalar_errors::GRAMMAR.permits(&ok, &err) => {
            Some(scalar_errors::result_boundary(facts)?)
        }
        _ => None,
    };
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Fresh concrete Results require their registered capability".into());
    }

    constructors::verify_core(facts)?;
    let result = match scalar_boundary {
        Some(boundary) => boundary.result.text()?,
        None => crate::data_types::token(ty)?,
    };
    if facts.signature.return_ty.normalized.as_deref() != Some(result.as_str()) {
        return Err("Fresh result interface disagrees with captured return type".into());
    }

    let source: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    crate::source_evidence::closed_body(&source.block)?;
    let error = TypeToken::from_tree(error);
    reject_storage(
        facts,
        |token| types::contains_owned(token, &error),
        "Result/error parameters and local storage require move/liveness proof",
    )?;
    fresh(&facts.body, true, &error, facts)
}

fn verify_transport(facts: &dyn EvidenceFacts) -> Result<(), String> {
    crate::result_maps::verify_chain(facts)?
        .ok_or_else(|| "Generated Result requires its complete constructor transport proof".into())
        .map(|_| ())
}

fn transport_interface(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if !facts
        .build
        .frontend_options
        .contains_key(crate::concrete_types::KEY)
    {
        return Ok(false);
    }
    let interface = facts
        .build
        .frontend_options
        .data::<crate::concrete_types::InterfaceObservations>()?;
    Ok(matches!(&interface.result,
        crate::concrete_types::Observation::Captured { ty: TypeTree::Result { ok, err } }
        if crate::result_maps::types::tree_result(ok, err)))
}

fn reject_storage(
    facts: &dyn EvidenceFacts,
    forbidden: impl Fn(&TypeToken) -> bool,
    diagnostic: &str,
) -> Result<(), String> {
    for ty in facts
        .signature
        .params
        .iter()
        .map(|p| &p.ty)
        .chain(facts.bindings.iter().map(|b| &b.ty))
    {
        let token = ty
            .normalized
            .as_deref()
            .ok_or("Missing fresh result binding type")?;
        if forbidden(&crate::data_types::parse(token)?) {
            return Err(diagnostic.into());
        }
    }
    Ok(())
}

fn fresh(
    node: &HirNode,
    returning: bool,
    error: &TypeToken,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if borrowed_fresh(node, returning, error, facts)? {
        return Ok(());
    }
    if crate::product_result_propagation::candidate(node, facts) {
        let checked = crate::product_result_propagation::verify(node, facts)?;
        return fresh(checked.operand, true, error, facts);
    }
    if crate::scalar_result_propagation::candidate(node, facts) {
        let checked = crate::scalar_result_propagation::verify(node, facts)?;
        return fresh(checked.operand, true, error, facts);
    }
    if crate::result_propagation::candidate(node, facts) {
        let checked = crate::result_propagation::verify(node, facts)?;
        return fresh(checked.operand, true, error, facts);
    }

    // An explicit return owns its operand even inside a non-tail branch.
    // Historical Result families keep their original boundary.
    let returning = returning
        || node.kind == "expr_ret"
            && scalar_errors::error(error).is_some()
            && crate::extraction_profile::facts_support(facts, scalar_errors::CAPABILITY);
    verify_fresh_use(node, returning, error, facts)?;

    let constructor = constructor_candidate(node, facts);
    if constructor {
        verify_constructor(node, facts)?;
    }

    for (position, child) in node.children.iter().enumerate() {
        let tail = returning && node.kind == "expr_block" && position + 1 == node.children.len();
        fresh(
            child,
            tail || node.kind == "expr_ret" || returning && constructor,
            error,
            facts,
        )?;
    }
    Ok(())
}

fn verify_fresh_use(
    node: &HirNode,
    returning: bool,
    error: &TypeToken,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let token = node
        .ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        .map(crate::data_types::parse)
        .transpose()?;

    if token
        .as_ref()
        .is_some_and(|ty| types::contains_owned(ty, error))
        && (!returning
            || !(matches!(
                node.kind.as_str(),
                "expr_call" | "expr_struct" | "expr_block" | "expr_ret"
            ) || method_result(node)
                && crate::extraction_profile::facts_support(
                    facts,
                    crate::result_propagation::CAPABILITY,
                )))
    {
        return Err(format!(
            "Result/error {} at {}..{} is not a fresh constructor/helper result forwarded whole (returning={returning})",
            node.kind, node.span.start, node.span.end,
        ));
    }

    Ok(())
}

fn borrowed_fresh(
    node: &HirNode,
    returning: bool,
    error: &TypeToken,
    facts: &dyn EvidenceFacts,
) -> Result<bool, String> {
    if crate::borrowed_result_propagation::candidate(node, facts) {
        let checked = crate::borrowed_result_propagation::verify(node, facts)?;
        fresh(checked.operand, true, error, facts)?;
        return Ok(true);
    }
    if crate::borrowed_result_errors::candidate(node, facts) {
        if !returning {
            return Err("Fresh borrowed Result cannot be ignored or stored".into());
        }
        let checked = crate::borrowed_result_errors::verify(node, facts)?;
        fresh(checked.receiver, false, error, facts)?;
        fresh(checked.error, true, error, facts)?;
        return Ok(true);
    }
    if crate::borrowed_result_errors::error_candidate(node, facts) {
        if !returning {
            return Err("Fresh error cannot be ignored or stored".into());
        }
        let checked = crate::borrowed_result_errors::verify_error(node, facts)?;
        if &checked.token != error {
            return Err("Fresh error differs from the complete return error".into());
        }
        return Ok(true);
    }
    Ok(false)
}

fn method_result(node: &HirNode) -> bool {
    node.kind == "expr_method_call"
        && crate::data_aggregates::parsed_type(node)
            .is_ok_and(|token| matches!(token, TypeToken::Result { .. }))
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_capability(
        crate::extraction_profile::policy_supports(profile, crate::result_maps::CAPABILITY),
        || transport_interface(facts),
        "Constructor Result transport requires its registered document capability",
    )?;
    fresh_products::verify_document_policy(profile, facts)?;
    verify_copy_document_policy(profile, facts)?;
    crate::extraction_profile::require_capability(
        crate::extraction_profile::policy_supports(profile, scalar_errors::CAPABILITY),
        || types::scalar_used(facts),
        "Fresh scalar error Result requires its registered document capability",
    )?;

    crate::extraction_profile::require_capability(
        crate::extraction_profile::policy_supports(profile, CAPABILITY),
        || types::used(facts),
        "Fresh concrete Results require their registered document capability",
    )?;
    verify_borrowed_document_policy(profile, facts)?;
    crate::extraction_profile::require_document(
        profile,
        crate::result_propagation::CAPABILITY,
        || {
            crate::field_locations::operations(&facts.body, &["expr_method_call"])
                .into_iter()
                .any(|node| {
                    method_result(node) && !crate::borrowed_result_errors::candidate(node, facts)
                })
        },
        "Fresh Result method forwarding requires its registered document capability",
    )
}

fn verify_borrowed_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        crate::borrowed_result_types::CAPABILITY,
        || {
            facts
                .signature
                .return_ty
                .normalized
                .as_deref()
                .and_then(|text| crate::data_types::parse(text).ok())
                .is_some_and(|token| {
                    crate::borrowed_result_types::uses_result(&token)
                        || crate::borrowed_result_types::error(&token).is_some()
                })
        },
        "Borrowed Result boundary requires its registered document capability",
    )
}

fn verify_copy_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        copy_errors::CAPABILITY,
        || {
            facts
                .signature
                .return_ty
                .normalized
                .as_deref()
                .and_then(|text| crate::data_types::parse(text).ok())
                .is_some_and(|token| copy_errors::contains_copy_result(&token))
        },
        "Copy fieldless Result requires its registered document capability",
    )
}

/// Whole fresh no-drop values can cross the helper boundary without Copy.
pub(crate) fn fresh_result(facts: &dyn EvidenceFacts, pair: crate::producer::PairView<'_>) -> bool {
    if fresh_products::interface_used(facts).unwrap_or(true) {
        return crate::helper_component_outputs::fresh_product_result(facts, pair);
    }
    pair.cfg.extraction_evidence.status == crate::ExtractionStatus::Complete
        && types::used(facts).unwrap_or(false)
        && verify_function(facts).is_ok()
}
