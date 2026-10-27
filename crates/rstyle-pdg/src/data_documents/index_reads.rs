//! Serialized scalar index reads and access-relative element selectors.

use super::*;
use crate::{
    Access, MemoryLocation, PdgNode,
    data_index_reads::{self, Checked},
    producer::PairView,
};

pub(super) fn install(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    super::subslice_views::install(pair, context.facts)?;
    super::read_accesses::install(pair, context, super::read_accesses::Kind::Index)
}

pub(super) fn verify(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let facts = context.facts;
    let (views, sources): (Vec<_>, Vec<_>) =
        crate::field_locations::operations(&facts.body, &["expr_index"])
            .into_iter()
            .partition(|source| crate::data_subslices::candidate(source, facts));
    super::subslice_views::verify(pair, facts, views)?;
    let serialized = pair.pdg.nodes.iter().filter(|node| operation(node)).count();
    if serialized != sources.len() {
        return Err("Index operation inventory disagrees with source".into());
    }

    let mut locations = BTreeMap::new();
    for source in sources {
        let checked = data_index_reads::verify(source, facts)?;
        let node = super::serialized_operation(pair, &source.span, operation)?;
        let derived = checked.locations(facts)?;
        let (reads, entries) = context.read_accesses(derived)?;
        verify_effects(node, &checked, &reads)?;
        verify_operands(pair, node, &checked, source)?;
        locations.extend(entries);
    }

    Ok(locations)
}

fn verify_effects(node: &PdgNode, checked: &Checked<'_>, reads: &[Access]) -> Result<(), String> {
    let (category, construct) = if checked.shared {
        (NodeCategory::Read, "data.index")
    } else {
        (NodeCategory::Computation, "data.value-index")
    };
    let bounds = checked.bounds()?;
    if node.category != category
        || node.attributes.construct.as_deref() != Some(construct)
        || node.attributes.reads != reads
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.operator.is_some()
        || node.attributes.literal.is_some()
        || node.attributes.callee.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
        || node.annotations.get(data_index_reads::BOUNDS_KEY) != Some(&bounds)
    {
        return Err(
            "Index operation loses its exact scalar, bounds or memory interpretation".into(),
        );
    }
    Ok(())
}

fn verify_operands(
    pair: PairView<'_>,
    node: &PdgNode,
    checked: &Checked<'_>,
    source: &crate::HirNode,
) -> Result<(), String> {
    let base = super::value_operation(pair, checked.base)?;
    let index = super::value_operation(pair, checked.index)?;
    let mut actual = Vec::new();
    for edge in pair.pdg.edges.iter().filter(|edge| edge.target == node.id) {
        if edge.subkind == "reaching_definition"
            && !(crate::extraction_profile::policy_supports(
                &pair.pdg.profile,
                crate::mutable_scalars::CAPABILITY,
            ) && edge.location_id.is_some()
                && edge.symbol_id.is_none())
        {
            return Err("Indexed read invents a lexical read".into());
        }
        if edge.kind == crate::EdgeFamily::Data && edge.subkind == "value_operand" {
            actual.push((edge.operand_position, &edge.source));
        }
    }

    verify_bounds_check(pair, source, &[base, index])?;
    actual.sort();
    if actual != [(Some(0), &base.id), (Some(1), &index.id)] {
        return Err("Index operation loses its ordered base/index snapshot operands".into());
    }
    Ok(())
}

fn verify_bounds_check(
    pair: PairView<'_>,
    source: &crate::HirNode,
    values: &[&PdgNode],
) -> Result<(), String> {
    if !crate::extraction_profile::policy_supports(
        &pair.pdg.profile,
        crate::mutable_scalars::CAPABILITY,
    ) || !pair.pdg.extensions.contains_key(super::memory_flow::KEY)
    {
        return Ok(());
    }
    let check = super::typed_operation(pair, source, "data.mutable-index-check", "unit")?;
    if check.category != NodeCategory::Computation
        || !check.attributes.reads.is_empty()
        || !check.attributes.writes.is_empty()
        || !check.attributes.defines.is_empty()
        || !check.attributes.symbol_ids.is_empty()
        || check.attributes.operator.is_some()
        || check.attributes.literal.is_some()
        || check.attributes.callee.is_some()
        || check
            .attributes
            .extensions
            .get("compat.rust.hir-r0:source-operation")
            .and_then(|source| source.get("role"))
            .and_then(serde_json::Value::as_str)
            != Some("index_bounds_check")
    {
        return Err("Mutable index check changes its source, role or memory effects".into());
    }
    super::verify_saved_operands(pair, check, values)
}

fn operation(node: &PdgNode) -> bool {
    !matches!(
        node.attributes.construct.as_deref(),
        Some("data.subslice-place" | "data.mutable-index-check")
    ) && matches!(
        node.category,
        NodeCategory::Read | NodeCategory::Computation
    ) && node.original_kind.known_value() == Some("expr_index")
}
