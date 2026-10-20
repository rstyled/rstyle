//! Independent source-to-document checked Option interpretation.
mod guards;

use super::*;
use crate::{
    HirNode, PdgNode, checked_options as options, producer::PairView, unit_store::EvidenceFacts,
};

const CONSTRUCTS: [&str; 7] = [
    "data.option-get",
    "data.option-payload",
    "match.option-some",
    "match.option-none",
    "data.option-some",
    "data.option-none",
    "data.option-value-payload",
];

pub(super) fn install(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    install_semantics(&mut pair.pdg.nodes);
    super::read_accesses::install(pair, context, super::read_accesses::Kind::Option)
}

fn install_semantics(nodes: &mut [PdgNode]) {
    for node in nodes {
        if let Some(construct) = &node.attributes.construct
            && CONSTRUCTS.contains(&construct.as_str())
        {
            node.annotations
                .insert(options::KEY.into(), options::semantics(construct));
        }
    }
}

pub(super) fn verify(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
    calls: &[&HirNode],
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    let facts = context.facts;
    let gets: Vec<_> = calls
        .iter()
        .copied()
        .filter(|source| options::get_candidate(source, facts))
        .collect();
    let matches = options::matches(facts);

    let constructors = options::constructors(facts);
    verify_inventory(pair, &gets, &matches, &constructors)?;
    for source in constructors {
        verify_constructor(pair, source, facts)?;
    }
    for source in gets {
        verify_get(pair, source, facts)?;
    }
    let mut locations = BTreeMap::new();
    for source in matches {
        locations.extend(verify_match(pair, context, source)?);
    }
    Ok(locations)
}

fn verify_inventory(
    pair: PairView<'_>,
    gets: &[&HirNode],
    matches: &[&HirNode],
    constructors: &[&HirNode],
) -> Result<(), String> {
    let scalar = matches
        .iter()
        .filter(|source| options::owned(&source.children[0]))
        .count();
    let some = constructors
        .iter()
        .filter(|source| source.kind == "expr_call")
        .count();
    inventory(
        pair,
        [
            gets.len(),
            matches.len() - scalar,
            matches.len(),
            matches.len(),
            some,
            constructors.len() - some,
            scalar,
        ],
    )
}

fn verify_constructor(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let construct = options::verify_constructor(source, facts)?;
    let node = operation(pair, source, construct)?;
    effects(node, construct, &[])?;

    let operands = source
        .children
        .iter()
        .map(|child| super::value_operation(pair, child))
        .collect::<Result<Vec<_>, _>>()?;
    super::verify_saved_operands(pair, node, &operands)
}

fn verify_match(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
    source: &HirNode,
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    let facts = context.facts;

    let checked = options::verify_match(source, facts)?;
    let construct = if checked.scalar_value {
        "data.option-value-payload"
    } else {
        "data.option-payload"
    };
    let load = operation(pair, checked.binding, construct)?;
    let (reads, entries) = if checked.scalar_value {
        (Vec::new(), BTreeMap::new())
    } else {
        context.read_accesses(options::locations(checked.binding, facts)?)?
    };
    effects(load, construct, &reads)?;
    super::verify_saved_operands(
        pair,
        load,
        &[super::value_operation(pair, checked.scrutinee)?],
    )?;
    verify_binding(pair, &checked, load)?;
    for (index, arm) in source.children[1..].iter().enumerate() {
        let construct = if index == checked.some_arm {
            "match.option-some"
        } else {
            "match.option-none"
        };
        let selector = operation(pair, &arm.children[0], construct)?;
        effects(selector, construct, &[])?;
        super::verify_saved_operands(
            pair,
            selector,
            &[super::value_operation(pair, checked.scrutinee)?],
        )?;
        if index == checked.some_arm {
            guards::verify(pair, selector, load)?;
        }
    }
    Ok(entries)
}

fn inventory(pair: PairView<'_>, expected_counts: [usize; 7]) -> Result<(), String> {
    for (construct, expected) in CONSTRUCTS.into_iter().zip(expected_counts) {
        if pair
            .pdg
            .nodes
            .iter()
            .filter(|node| node.attributes.construct.as_deref() == Some(construct))
            .count()
            != expected
        {
            return Err("Checked Option operation inventory disagrees with source".into());
        }
    }
    if pair.pdg.nodes.iter().any(|node| {
        node.annotations.contains_key(options::KEY)
            && node
                .attributes
                .construct
                .as_deref()
                .is_none_or(|construct| !CONSTRUCTS.contains(&construct))
    }) {
        return Err("Orphan checked Option interpretation".into());
    }
    Ok(())
}

fn operation<'a>(
    pair: PairView<'a>,
    source: &HirNode,
    construct: &str,
) -> Result<&'a PdgNode, String> {
    let node = super::serialized_operation(pair, &source.span, |node| {
        node.attributes.construct.as_deref() == Some(construct)
    })?;
    let expected = if matches!(construct, "match.option-some" | "match.option-none") {
        "bool"
    } else {
        crate::data_aggregates::token(source)?
    };
    if node.original_kind.known_value() != Some(source.kind.as_str())
        || node
            .annotations
            .get("compat.rust.hir-r0:type")
            .and_then(|ty| ty.get("value"))
            .and_then(serde_json::Value::as_str)
            != Some(expected)
    {
        return Err("Option operation type/source correspondence disagrees".into());
    }
    Ok(node)
}

fn verify_get(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let checked = options::verify_get(source, facts)?;
    let node = operation(pair, source, "data.option-get")?;
    effects(node, "data.option-get", &[])?;
    super::verify_saved_operands(
        pair,
        node,
        &[
            super::value_operation(pair, checked.receiver)?,
            super::value_operation(pair, checked.index)?,
        ],
    )?;

    super::summary_records::Record {
        source,
        node,
        summary: options::resolve(source, facts)?,
    }
    .verify(pair, facts)
}

fn effects(node: &PdgNode, construct: &str, reads: &[crate::Access]) -> Result<(), String> {
    let expected = match construct {
        "data.option-get" => NodeCategory::Call,
        "data.option-payload" => NodeCategory::Read,
        "data.option-some" | "data.option-none" | "data.option-value-payload" => {
            NodeCategory::Computation
        }
        _ => NodeCategory::Control,
    };
    if node.category != expected
        || node.annotations.get(options::KEY) != Some(&options::semantics(construct))
        || node.attributes.reads != reads
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.literal.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Checked Option loses exact discriminant, conditional read or effects".into());
    }
    if expected != NodeCategory::Call
        && (node.attributes.operator.is_some() || node.attributes.callee.is_some())
    {
        return Err("Checked Option operation invents a call/operator".into());
    }
    Ok(())
}

fn verify_binding(
    pair: PairView<'_>,
    checked: &options::Match<'_>,
    load: &PdgNode,
) -> Result<(), String> {
    let declaration = super::serialized_operation(pair, &checked.binding.span, |node| {
        node.category == NodeCategory::Declaration
    })?;
    let symbol = checked
        .binding
        .binding
        .as_ref()
        .ok_or("Missing Option scalar binding")?;
    if declaration.attributes.symbol_ids != [symbol.clone()]
        || declaration.attributes.defines.len() != 1
        || declaration.attributes.writes.len() != 1
        || declaration.attributes.writes[0].symbol_id.as_ref() != Some(symbol)
        || declaration.attributes.writes[0].location_id.is_some()
        || !declaration.attributes.reads.is_empty()
        || declaration.attributes.write_form.as_deref() != Some("initialization")
    {
        return Err("Option payload does not define its exact scalar binding".into());
    }
    super::verify_saved_operands(pair, declaration, &[load])
}
