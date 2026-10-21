//! Independent checks for the complete fresh iterator/prefix/result composition.
use super::*;
use crate::iterator_position as contract;

const CONSTRUCTS: [&str; 6] = [
    "iterator.fresh-shared-bytes",
    "iterator.shared-byte-environment",
    "iterator.nonempty",
    contract::PREFIX,
    "iterator.empty-position",
    "iterator.position-result",
];

pub(super) fn install(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    contract::sources(context.facts)
        .into_iter()
        .try_fold(BTreeMap::new(), |locations, source| {
            crate::helper_reads::merge_locations(locations, install_source(pair, context, source)?)
        })
}

fn install_source(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
    source: &crate::HirNode,
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    let (reads, entries) = context.read_accesses(contract::locations(source, context.facts)?)?;
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| {
            node.attributes.construct.as_deref() == Some(contract::PREFIX)
                && node.source_span.as_ref().is_some_and(|span| {
                    span.start == u64::from(source.span.start)
                        && span.end == u64::from(source.span.end)
                })
        })
        .ok_or("Missing iterator prefix operation")?;

    install_prefix(node, reads);
    Ok(entries)
}

fn install_prefix(node: &mut PdgNode, reads: Vec<crate::Access>) {
    node.attributes.reads = reads;
    node.annotations
        .insert(contract::KEY.into(), contract::semantics());
}

pub(super) fn verify(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, crate::MemoryLocation>, String> {
    let sources = contract::sources(context.facts);
    super::verify_construct_inventory(pair, CONSTRUCTS, [sources.len(); 6])?;
    if !sources.is_empty()
        && !crate::extraction_profile::policy_supports(&pair.pdg.profile, contract::CAPABILITY)
    {
        return Err("Byte iterator requires its registered document policy".into());
    }
    let mut locations = BTreeMap::new();

    for source in sources {
        let checked = contract::verify(source, context.facts)?;
        let prefix = super::serialized_operation(pair, &source.span, |node| {
            node.attributes.construct.as_deref() == Some(contract::PREFIX)
        })?;
        let (reads, entries) =
            context.read_accesses(contract::locations(source, context.facts)?)?;
        if prefix.category != NodeCategory::Read
            || prefix.original_kind.known_value() != Some("rstyle.iterator-prefix")
            || super::origins::wire_type(prefix)? != contract::RESULT_TOKEN
            || prefix.attributes.reads != reads
            || prefix.annotations.get(contract::KEY) != Some(&contract::semantics())
        {
            return Err("Iterator prefix loses conditional repeated-read semantics".into());
        }
        effects(prefix, &reads)?;
        verify_flow(pair, &checked, prefix)?;
        locations.extend(entries);
    }
    Ok(locations)
}

fn verify_flow(
    pair: PairView<'_>,
    checked: &contract::Checked<'_>,
    prefix: &PdgNode,
) -> Result<(), String> {
    let created = super::typed_operation(pair, checked.iterator, CONSTRUCTS[0], "unit")?;
    let environment = super::typed_operation(pair, checked.callback, CONSTRUCTS[1], "unit")?;
    let selector = super::typed_operation(pair, checked.call, CONSTRUCTS[2], "bool")?;
    let empty = super::typed_operation(
        pair,
        &checked.callback.children[0],
        CONSTRUCTS[4],
        contract::RESULT_TOKEN,
    )?;
    let receiver = super::value_operation(pair, checked.receiver)?;
    let capture = super::value_operation(pair, checked.predicate.captured_value)?;
    let result = super::value_operation(pair, checked.call)?;

    for node in [created, environment, selector, empty, result] {
        effects(node, &[])?;
    }
    if created.category != NodeCategory::Computation
        || environment.category != NodeCategory::Computation
        || selector.category != NodeCategory::Control
        || empty.category != NodeCategory::Computation
        || result.category != NodeCategory::Merge
        || result.attributes.construct.as_deref() != Some(CONSTRUCTS[5])
    {
        return Err("Iterator creation/selection/result categories disagree".into());
    }
    for (node, operands) in [
        (created, vec![receiver]),
        (environment, vec![]),
        (selector, vec![receiver]),
        (empty, vec![]),
        (prefix, vec![receiver, capture]),
    ] {
        super::verify_saved_operands(pair, node, &operands)?;
    }
    order(pair, [receiver, created, environment, selector])?;
    super::conditional_payloads::verify(pair, selector, prefix)?;
    super::conditional_payloads::verify(pair, selector, capture)?;
    super::conditional_payloads::guarded(pair, selector, empty, crate::CfgEdgeKind::False)?;
    super::callbacks::order::before(
        pair,
        &super::callbacks::order::completion(pair, capture)?,
        prefix,
    )?;
    verify_result(pair, selector, [prefix, empty], result)
}

fn verify_result(
    pair: PairView<'_>,
    selector: &PdgNode,
    values: [&PdgNode; 2],
    result: &PdgNode,
) -> Result<(), String> {
    let site = super::callbacks::order::site(pair, &result.id)?;
    let alternatives = site["alternatives"]
        .as_array()
        .ok_or("Missing iterator result alternatives")?;
    if site["controller_pdg_node_id"] != selector.id
        || alternatives.len() != 2
        || [("true", values[0]), ("false", values[1])]
            .into_iter()
            .any(|(outcome, expected)| {
                alternatives
                    .iter()
                    .filter(|alternative| {
                        alternative["outcome"] == outcome
                            && alternative["value_pdg_node_id"] == expected.id
                    })
                    .count()
                    != 1
            })
    {
        return Err("Iterator result loses prefix-on-nonempty/None-on-empty selection".into());
    }
    Ok(())
}

fn order(pair: PairView<'_>, nodes: [&PdgNode; 4]) -> Result<(), String> {
    for adjacent in nodes.windows(2) {
        super::callbacks::order::before(
            pair,
            &super::callbacks::order::completion(pair, adjacent[0])?,
            adjacent[1],
        )?;
    }
    Ok(())
}

fn effects(node: &PdgNode, reads: &[crate::Access]) -> Result<(), String> {
    if node.attributes.reads != reads
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.operator.is_some()
        || node.attributes.literal.is_some()
        || node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || node.attributes.write_form.is_some()
        || node.attributes.exit_kind.is_some()
        || node.attributes.position.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Iterator semantic operation invents effects, calls or definitions".into());
    }
    Ok(())
}
