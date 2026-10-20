//! Physical memory accesses selected through evaluated shared references.

use super::*;
use crate::{
    Access, HirNode, MemoryLocation, PdgNode, field_locations::derived, producer::PairView,
};

pub(super) fn install(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    if !pair.pdg.nodes.iter().any(|node| {
        matches!(
            node.attributes.construct.as_deref(),
            Some("data.indirect-field" | "data.indirect-deref")
        )
    }) {
        return Ok(BTreeMap::new());
    }

    let sources =
        crate::field_locations::operations(&context.facts.body, &["expr_field", "expr_unary"]);
    let places = crate::data_places::Places::verify(context.facts)?;
    let facts = context.facts;
    let entries = sources
        .into_iter()
        .filter(|source| derived::candidate(source, facts) && !places.contains(source))
        .map(|source| {
            let (reads, entries) = effects(source, context)?;
            install_read(pair, source, reads)?;
            Ok(entries)
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(entries.into_iter().flatten().collect())
}

fn install_read(pair: &mut Pair, source: &HirNode, reads: Vec<Access>) -> Result<(), String> {
    let node = super::serialized_operation_mut(pair, source)?;
    node.attributes.reads = reads;
    Ok(())
}

fn effects(
    source: &HirNode,
    context: &mut LocationContext<'_>,
) -> Result<(Vec<Access>, BTreeMap<String, MemoryLocation>), String> {
    let locations = derived::locations(source, context.facts)?;
    context.read_accesses(locations)
}

pub(super) fn verify(
    node: &PdgNode,
    source: &HirNode,
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let mut expected = BTreeMap::new();
    if node.annotations.contains_key(crate::data_places::KEY)
        || node.attributes.operator.is_some()
        || node.attributes.literal.is_some()
        || node.attributes.callee.is_some()
        || !node.attributes.symbol_ids.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.writes.is_empty()
    {
        return Err("Indirect field invents lexical, place or scalar effects".into());
    }

    let places = crate::data_places::Places::verify(context.facts)?;
    if places.contains(source) {
        if node.category != NodeCategory::Computation
            || node.attributes.construct.as_deref() != Some("data.indirect-place")
            || !node.attributes.reads.is_empty()
        {
            return Err("Indirect field place invents a memory read".into());
        }
    } else {
        let (reads, locations) = effects(source, context)?;
        if node.category != NodeCategory::Read
            || node.attributes.construct.as_deref() != Some(construct(source))
            || node.attributes.reads != reads
        {
            return Err("Indirect field read loses its exact possible locations".into());
        }
        expected.extend(locations);
    }
    if pair
        .pdg
        .edges
        .iter()
        .any(|edge| edge.target == node.id && edge.subkind == "reaching_definition")
    {
        return Err("Indirect memory read invents a lexical binding read".into());
    }
    let base = super::field_base(pair, context.facts, node)?;
    super::verify_field_value_edge(pair, node, base)?;
    Ok(expected)
}

fn construct(source: &HirNode) -> &'static str {
    if source.kind == "expr_unary" {
        "data.indirect-deref"
    } else {
        "data.indirect-field"
    }
}

pub(super) fn verify_dereferences(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let facts = context.facts;
    let mut locations = BTreeMap::new();
    for source in crate::field_locations::operations(&facts.body, &["expr_unary"])
        .into_iter()
        .filter(|source| crate::data_dereferences::candidate(source, facts))
    {
        let node = super::serialized_operation(pair, &source.span, |node| {
            node.original_kind.known_value() == Some("expr_unary")
                && matches!(
                    node.category,
                    NodeCategory::Read | NodeCategory::Computation
                )
        })?;
        locations.extend(verify(node, source, pair, context)?);
    }

    for node in &pair.pdg.nodes {
        if node.attributes.construct.as_deref() == Some("data.indirect-deref") {
            let source = super::origins::source_node(node, facts)?;
            crate::data_dereferences::verify(source, facts)?;
        }
    }
    Ok(locations)
}
