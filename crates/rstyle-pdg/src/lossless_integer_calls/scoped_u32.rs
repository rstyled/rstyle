//! Conversion authority for the complete named-error widening-reader proof.
use super::*;
use crate::data_library::Registry;

const CAPABILITY: &str = "rust.named-error-usize-reader-flow";

/// The owning reader verifier must independently prove its complete source/HIR
/// shape, expected eligible-call roster, callback transfers and graph edges.
/// Ordinary call resolution never falls back to this callback-only entry point.
pub(crate) fn verify(
    node: &HirNode,
    chain: &[&HirNode],
    facts: &dyn EvidenceFacts,
) -> Result<Summary, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || !crate::extraction_profile::facts_support(facts, U32_USIZE_CAPABILITY)
        || kind(node, facts) != Some(Kind::ConvertU32Usize)
    {
        return Err("Scoped reader requires the enabled exact u32-to-usize conversion".into());
    }
    let kind = Kind::ConvertU32Usize;
    kind.verify_types(node, facts, true)?;
    let registry = authority::verify(node, kind, facts)?;
    let inventory = facts
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Scoped reader conversion lacks original call instance inventory")?;
    let call = selection::scoped_site(inventory, node, chain)?;
    selection::relationship(call, kind, registry)?;
    source::verify(node, kind, facts)?;
    error_discard(registry)?;

    summary(node, kind)
}

fn error_discard(registry: &Registry) -> Result<(), String> {
    authority::sources(registry)?;
    if registry.definitions.get(ERROR).is_none_or(String::is_empty) {
        return Err(
            "Scoped reader discard lacks the pinned core TryFromIntError definition".into(),
        );
    }
    // The reviewed num/error.rs defines TryFromIntError(IntErrorKind), deriving
    // Copy for both. Copy excludes Drop, so wildcard discard has no destructor.
    // No error layout, unit field or inhabitance assumption is used. The exact
    // selected TryFrom impl and pinned convert/num.rs establish this error type.
    Ok(())
}

fn summary(node: &HirNode, kind: Kind) -> Result<Summary, String> {
    let (argument_types, result_type) = kind.signature()?;
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing scoped conversion dispatch")?;
    Ok(Summary {
        id: format!("rstyle.{}", kind.construct()),
        version: "1.0.0".into(),
        callee_path: dispatch.op.clone(),
        callee_crate: "core".into(),
        argument_types,
        result_type,
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: kind.overflow_behavior().into(),
    })
}
