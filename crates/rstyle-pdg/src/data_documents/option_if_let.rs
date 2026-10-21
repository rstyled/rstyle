//! Independent saved-initializer and success-only scalar binding checks.

use super::*;
use crate::{HirNode, checked_options, unit_store::EvidenceFacts};

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let sources = checked_options::if_lets(facts);
    super::verify_construct_inventory(pair, ["if.option-some"], [sources.len()])?;
    if !sources.is_empty()
        && !crate::extraction_profile::policy_supports(
            &pair.pdg.profile,
            "rust.scalar-option-if-let",
        )
    {
        return Err("Scalar Option if-let requires its serialized document policy".into());
    }

    for source in sources {
        invocation(pair, source, facts)?;
    }

    Ok(())
}

fn invocation(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let checked = checked_options::verify_if_let(source, facts)?;
    let selector = super::typed_operation(pair, checked.condition, "if.option-some", "bool")?;
    super::checked_options::effects(selector, "if.option-some", &[])?;

    let initializer = super::value_operation(pair, checked.scrutinee)?;
    super::verify_saved_operands(pair, selector, &[initializer])?;

    let payload =
        super::typed_operation(pair, checked.binding, "data.option-value-payload", "usize")?;
    super::checked_options::effects(payload, "data.option-value-payload", &[])?;
    super::verify_saved_operands(pair, payload, &[initializer])?;
    super::conditional_payloads::verify(pair, selector, payload)?;

    super::conditional_payloads::binding(pair, checked.binding, payload)?;
    branch_guard(
        pair,
        selector,
        checked.then_branch,
        crate::CfgEdgeKind::True,
    )?;
    branch_guard(
        pair,
        selector,
        checked.else_branch,
        crate::CfgEdgeKind::False,
    )
}

fn branch_guard(
    pair: PairView<'_>,
    selector: &PdgNode,
    branch: &HirNode,
    outcome: crate::CfgEdgeKind,
) -> Result<(), String> {
    for node in &pair.pdg.nodes {
        if node.source_span.as_ref().is_some_and(|span| {
            selector
                .source_span
                .as_ref()
                .is_some_and(|source| span.source_id == source.source_id)
                && span.start >= u64::from(branch.span.start)
                && span.end <= u64::from(branch.span.end)
        }) {
            super::conditional_payloads::guarded(pair, selector, node, outcome)?;
        }
    }
    Ok(())
}
