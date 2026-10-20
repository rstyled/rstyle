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
        verify_operands(pair, node, &checked)?;
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
) -> Result<(), String> {
    let base = super::value_operation(pair, checked.base)?;
    let index = super::value_operation(pair, checked.index)?;
    let mut actual = Vec::new();
    for edge in pair.pdg.edges.iter().filter(|edge| edge.target == node.id) {
        if edge.subkind == "reaching_definition" {
            return Err("Indexed read invents a lexical read".into());
        }
        if edge.kind == crate::EdgeFamily::Data && edge.subkind == "value_operand" {
            actual.push((edge.operand_position, &edge.source));
        }
    }

    actual.sort();
    if actual != [(Some(0), &base.id), (Some(1), &index.id)] {
        return Err("Index operation loses its ordered base/index snapshot operands".into());
    }
    Ok(())
}

fn operation(node: &PdgNode) -> bool {
    node.attributes.construct.as_deref() != Some("data.subslice-place")
        && matches!(
            node.category,
            NodeCategory::Read | NodeCategory::Computation
        )
        && node.original_kind.known_value() == Some("expr_index")
}
