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
        let checked = data_subslices::verify(source, facts)?;
        let node = super::serialized_operation_mut(pair, source)?;
        node.annotations
            .insert(data_subslices::BOUNDS_KEY.into(), checked.bounds().value()?);
    }
    Ok(())
}

fn operations(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::selected_operations(facts, &["expr_index"], data_subslices::candidate)
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
        verify_policy(pair, &checked)?;
        let node = super::serialized_operation(pair, &source.span, |node| {
            node.category == NodeCategory::Computation
        })?;
        if node.attributes.construct.as_deref() != Some("data.subslice-place")
            || node.annotations.get(data_subslices::BOUNDS_KEY) != Some(&checked.bounds().value()?)
            || node.annotations.contains_key(crate::data_places::KEY)
        {
            return Err("Subslice loses its view/bounds interpretation".into());
        }
        super::verify_value_effects(
            node,
            false,
            "Subslice view invents memory or scalar effects",
        )?;
        verify_operands(pair, node, checked.operands())?;
    }
    Ok(())
}

fn verify_operands<'a>(
    pair: PairView<'_>,
    node: &PdgNode,
    sources: impl IntoIterator<Item = &'a HirNode>,
) -> Result<(), String> {
    let values = sources
        .into_iter()
        .map(|source| super::value_operation(pair, source))
        .collect::<Result<Vec<_>, _>>()?;

    super::verify_saved_operands(pair, node, &values)
}

pub(super) fn verify_borrow(
    pair: PairView<'_>,
    node: &PdgNode,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    data_subslices::verify_borrow(source, facts)?;
    verify_operands(pair, node, [&source.children[0]])
}

fn verify_policy(pair: PairView<'_>, checked: &data_subslices::Checked<'_>) -> Result<(), String> {
    for (required, capability) in [
        (
            checked.requires_result_policy,
            data_subslices::RESULT_CAPABILITY,
        ),
        (
            checked.array_length.is_some(),
            data_subslices::ARRAY_CAPABILITY,
        ),
        (
            checked.requires_prefix_policy,
            data_subslices::PREFIX_CAPABILITY,
        ),
    ] {
        if required && !crate::extraction_profile::policy_supports(&pair.pdg.profile, capability) {
            return Err(format!(
                "Subslice requires registered document capability {capability}"
            ));
        }
    }
    Ok(())
}
