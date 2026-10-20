//! Existing memory-read accesses for fields selected through reference values.

use super::*;
use crate::{
    Access, HirNode, MemoryLocation, PdgNode, field_locations::derived, producer::PairView,
};

pub(super) fn install(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    if !pair
        .pdg
        .nodes
        .iter()
        .any(|node| node.attributes.construct.as_deref() == Some("data.indirect-field"))
    {
        return Ok(BTreeMap::new());
    }

    let sources = crate::field_locations::fields(&context.facts.body);
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
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| {
            node.original_kind.known_value() == Some("expr_field")
                && node.source_span.as_ref().is_some_and(|span| {
                    span.start == u64::from(source.span.start)
                        && span.end == u64::from(source.span.end)
                })
        })
        .ok_or("Missing indirect field read")?;
    node.attributes.reads = reads;
    Ok(())
}

fn effects(
    source: &HirNode,
    context: &mut LocationContext<'_>,
) -> Result<(Vec<Access>, BTreeMap<String, MemoryLocation>), String> {
    let entries = derived::locations(source, context.facts)?
        .into_iter()
        .map(|location| {
            context.formal_location(
                location.parameter,
                &location.path,
                crate::data_types::TypeToken::from_tree(&location.value),
            )
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let reads = entries
        .keys()
        .map(|id| Access {
            symbol_id: None,
            location_id: Some(id.clone()),
            unresolved_reason: None,
            operand_position: Some(0),
        })
        .collect();

    Ok((reads, entries))
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
            || node.attributes.construct.as_deref() != Some("data.indirect-field")
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
