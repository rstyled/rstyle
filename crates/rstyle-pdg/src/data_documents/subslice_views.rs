//! Independent source-backed slice-view graph verification.

use super::*;
use crate::{HirNode, PdgNode, data_subslices, producer::PairView, unit_store::EvidenceFacts};

pub(super) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !pair
        .pdg
        .nodes
        .iter()
        .any(|node| node.attributes.construct.as_deref() == Some("data.subslice-place"))
    {
        return Ok(());
    }

    for source in operations(facts) {
        data_subslices::verify(source, facts)?;
        let node = super::serialized_operation_mut(pair, source)?;
        node.annotations.insert(
            data_subslices::BOUNDS_KEY.into(),
            crate::data_bounds::Bounds::subslice().value()?,
        );
    }
    Ok(())
}

fn operations(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::operations(&facts.body, &["expr_index"])
        .into_iter()
        .filter(|source| data_subslices::candidate(source, facts))
        .collect()
}

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    sources: Vec<&HirNode>,
) -> Result<(), String> {
    if pair.pdg.nodes.iter().any(|node| {
        node.annotations.contains_key(data_subslices::BOUNDS_KEY)
            && node.attributes.construct.as_deref() != Some("data.subslice-place")
    }) {
        return Err("Orphan subslice bounds interpretation".into());
    }

    let actual = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some("data.subslice-place"))
        .count();
    if actual != sources.len() {
        return Err("Subslice view inventory disagrees with source".into());
    }

    for source in sources {
        let checked = data_subslices::verify(source, facts)?;
        let node = super::serialized_operation(pair, &source.span, |node| {
            node.category == NodeCategory::Computation
        })?;
        if node.attributes.construct.as_deref() != Some("data.subslice-place")
            || node.annotations.get(data_subslices::BOUNDS_KEY)
                != Some(&crate::data_bounds::Bounds::subslice().value()?)
            || node.annotations.contains_key(crate::data_places::KEY)
        {
            return Err("Subslice loses its view/bounds interpretation".into());
        }
        super::verify_value_effects(
            node,
            false,
            "Subslice view invents memory or scalar effects",
        )?;
        verify_operands(pair, node, &[checked.base, checked.start, checked.end])?;
    }
    Ok(())
}

fn verify_operands(pair: PairView<'_>, node: &PdgNode, sources: &[&HirNode]) -> Result<(), String> {
    let expected = sources
        .iter()
        .enumerate()
        .map(|(position, source)| {
            Ok((
                Some(position as u64),
                super::value_operation(pair, source)?.id.as_str(),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut actual: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| {
            edge.target == node.id
                && edge.kind == crate::EdgeFamily::Data
                && edge.subkind == "value_operand"
        })
        .map(|edge| (edge.operand_position, edge.source.as_str()))
        .collect();

    actual.sort();
    if actual != expected
        || pair
            .pdg
            .edges
            .iter()
            .any(|edge| edge.target == node.id && edge.subkind == "reaching_definition")
    {
        return Err("Subslice loses ordered base/endpoint snapshots".into());
    }
    Ok(())
}

pub(super) fn verify_borrow(
    pair: PairView<'_>,
    node: &PdgNode,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    data_subslices::verify_borrow(source, facts)?;
    verify_operands(pair, node, &[&source.children[0]])
}
