//! Independently bind complete cast inventory, interpretation and saved operand.
use super::*;
use crate::{HirNode, bool_usize as cast, unit_store::EvidenceFacts};

pub(super) fn sources(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::selected_operations(facts, &["expr_cast"], |node, _| {
        cast::candidate(node)
    })
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let sources = sources(facts);
    super::verify_construct_inventory(pair, [cast::CONSTRUCT], [sources.len()])?;
    let actual: BTreeSet<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.annotations.contains_key(cast::KEY))
        .map(|node| node.id.as_str())
        .collect();

    let expected: BTreeSet<_> = sources
        .iter()
        .map(|source| {
            super::typed_operation(pair, source, cast::CONSTRUCT, "usize")
                .map(|node| node.id.as_str())
        })
        .collect::<Result<_, _>>()?;
    if actual != expected {
        return Err("Bool cast interpretation inventory disagrees with source".into());
    }
    if !sources.is_empty()
        && !crate::extraction_profile::policy_supports(&pair.pdg.profile, cast::CAPABILITY)
    {
        return Err("Bool cast requires its registered serialized document policy".into());
    }

    for source in sources {
        verify_operation(pair, source, facts)?;
    }
    Ok(())
}

fn verify_operation(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let node = super::typed_operation(pair, source, cast::CONSTRUCT, "usize")?;
    if node.category != NodeCategory::Computation
        || node
            .attributes
            .operator
            .as_ref()
            .and_then(|value| value.known_value())
            != Some("as")
        || node.annotations.get(cast::KEY) != Some(&cast::semantics(source, facts)?)
    {
        return Err(
            "Bool cast loses its exact false/true mapping or builtin interpretation".into(),
        );
    }

    effects(node)?;
    let operand = super::value_operation(pair, &source.children[0])?;
    super::verify_saved_operands(pair, node, &[operand])
}

fn effects(node: &PdgNode) -> Result<(), String> {
    if node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || node.attributes.literal.is_some()
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || !node.attributes.outcomes.is_empty()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Bool cast invents memory, call, literal, binding or control effects".into());
    }
    Ok(())
}
