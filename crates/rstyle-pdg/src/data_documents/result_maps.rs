//! Independent replay of tag guards, exact constructors and conserved payloads.
use super::value_consumers::verify as consumers;
use super::*;
use crate::{
    CfgEdgeKind,
    result_maps::{self, CheckedMap},
    unit_store::EvidenceFacts,
};

const KEY: &str = "rstyle.data:constructor-result-maps";
#[cfg(test)]
mod tests;
const CONSTRUCTS: [&str; 6] = [
    "map.result-is-err",
    "map.result-ok-payload",
    "map.result-err-payload",
    "map.constructor-field",
    "map.result-wrap-ok",
    "map.result-wrap-err",
];

struct Operations<'a> {
    saved: &'a PdgNode,
    selector: &'a PdgNode,
    payloads: [&'a PdgNode; 2],
    constructor: &'a PdgNode,
    results: [&'a PdgNode; 2],
    merge: &'a PdgNode,
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        result_maps::CAPABILITY,
        || facts.body.any(&|node| result_maps::candidate(node, facts)),
        "Constructor Result maps require their registered document policy",
    )
}

pub(super) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let Some(checked) = result_maps::verify_chain(facts)? else {
        return Ok(());
    };
    for stage in &checked.stages {
        let metadata = metadata(stage)?;
        for node in &mut pair.pdg.nodes {
            if operation_for(node, stage) {
                node.annotations.insert(KEY.into(), metadata.clone());
            }
        }
    }
    Ok(())
}

fn operation_for(node: &PdgNode, checked: &CheckedMap<'_>) -> bool {
    let Some(construct) = node.attributes.construct.as_deref() else {
        return false;
    };
    let source = if construct == CONSTRUCTS[3] {
        checked.callback
    } else {
        checked.source
    };
    role(node) == Some(checked.role())
        && CONSTRUCTS.contains(&construct)
        && node.source_span.as_ref().is_some_and(|span| {
            span.start == u64::from(source.span.start) && span.end == u64::from(source.span.end)
        })
}

pub(super) fn source_node<'a>(
    node: &PdgNode,
    facts: &'a dyn EvidenceFacts,
) -> Option<Result<&'a crate::HirNode, String>> {
    let semantic = map_construct(node);
    let argument = role(node) == Some("source_operation")
        && node.category == NodeCategory::Read
        && node.original_kind.known_value() == Some("expr_path");
    if !semantic && (!argument || !result_maps::tail_candidate(facts)) {
        return None;
    }

    let checked = match result_maps::verify_chain(facts) {
        Ok(Some(checked)) => checked,
        Ok(None) => return None,
        Err(reason) => return Some(Err(reason)),
    };

    Some(if semantic {
        stage_source(node, &checked.stages)
    } else {
        argument_source(node, facts, checked.stages[0].receiver)
    })
}

fn map_construct(node: &PdgNode) -> bool {
    matches!(node.attributes.construct.as_deref(), Some(construct)
        if CONSTRUCTS.contains(&construct) || construct == result_maps::RESULT)
}

fn stage_source<'a>(
    node: &PdgNode,
    stages: &[CheckedMap<'a>; 2],
) -> Result<&'a crate::HirNode, String> {
    let stage = stages
        .iter()
        .find(|stage| role(node) == Some(stage.role()))
        .ok_or("Result map operation has no exact checked stage role")?;
    let source = if node.attributes.construct.as_deref() == Some(CONSTRUCTS[3]) {
        stage.callback
    } else {
        stage.source
    };
    check_source_node(node, source)?;
    Ok(source)
}

fn check_source_node(node: &PdgNode, source: &crate::HirNode) -> Result<(), String> {
    let span = super::aggregates::source_span(node)?;
    let merge = node.attributes.construct.as_deref() == Some(result_maps::RESULT);
    if span.start != u64::from(source.span.start)
        || span.end != u64::from(source.span.end)
        || if merge {
            node.category != NodeCategory::Merge
        } else {
            node.original_kind.known_value() != Some(source.kind.as_str())
        }
    {
        return Err("Result map operation loses its exact checked source role/span/kind".into());
    }
    Ok(())
}

fn argument_source<'a>(
    node: &PdgNode,
    facts: &dyn EvidenceFacts,
    leaf: &'a crate::HirNode,
) -> Result<&'a crate::HirNode, String> {
    let [argument] = leaf.children.as_slice() else {
        return Err("Result transport leaf loses its sole checked argument".into());
    };
    let parameter = facts
        .signature
        .params
        .first()
        .ok_or("Result transport argument lacks its exact parameter")?;

    if node.attributes.construct.is_some()
        || node.attributes.symbol_ids != [parameter.symbol.clone()]
        || node.attributes.reads.len() != 1
        || node.attributes.reads[0].symbol_id.as_ref() != Some(&parameter.symbol)
        || node.attributes.reads[0].location_id.is_some()
        || !node.attributes.defines.is_empty()
        || !node.attributes.writes.is_empty()
    {
        return Err("Result transport argument read changes its exact parameter symbol".into());
    }

    check_source_node(node, argument)?;
    Ok(argument)
}

pub(super) fn verify_type(node: &PdgNode, facts: &dyn EvidenceFacts) -> Option<Result<(), String>> {
    if !map_construct(node) {
        return None;
    }
    Some((|| {
        let checked = result_maps::verify_chain(facts)?
            .ok_or("Result map value type lacks the complete checked chain")?;
        stage_source(node, &checked.stages)?;
        let stage = checked
            .stages
            .iter()
            .find(|stage| role(node) == Some(stage.role()))
            .ok_or("Result map value type has no exact checked stage")?;
        let construct = node.attributes.construct.as_deref().unwrap_or_default();
        let expected = if construct == CONSTRUCTS[0] {
            "bool".into()
        } else {
            let tree = match construct {
                "map.result-ok-payload" => stage.payload_types()[0],
                "map.result-err-payload" => stage.payload_types()[1],
                "map.constructor-field" => stage.constructed_type(),
                _ => &stage.output,
            };
            crate::data_types::token(tree)?
        };
        if super::origins::wire_type(node)? != expected {
            return Err("Result map value type disagrees with its exact checked stage role".into());
        }
        Ok(())
    })())
}

#[derive(serde::Serialize)]
struct Metadata<'a> {
    identity: &'static str,
    input: String,
    output: String,
    constructor: &'a crate::HirRef,
    source: &'a crate::HirRef,
    maps_error: bool,
    selector: &'static str,
    payload: &'static str,
    constructor_rule: &'static str,
    unchanged_branch: &'static str,
}

fn metadata(checked: &CheckedMap<'_>) -> Result<serde_json::Value, String> {
    let receipt = Metadata {
        identity: "rstyle.rust.constructor-result-transport/1",
        input: crate::data_types::token(&checked.input)?,
        output: crate::data_types::token(&checked.output)?,
        constructor: &checked.callback.hir,
        source: &checked.source.hir,
        maps_error: checked.maps_error,
        selector: "inspect-discriminant-without-moving/1",
        payload: "move-selected-payload-once/1",
        constructor_rule: "actual-unary-struct-constructor/1",
        unchanged_branch: "rewrap-same-payload-in-new-result/1",
    };

    serde_json::to_value(receipt).map_err(|error| error.to_string())
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    verify_document_policy(&pair.pdg.profile, facts)?;
    let checked = result_maps::verify_chain(facts)?;
    let count = checked.as_ref().map_or(0, |checked| checked.stages.len());
    super::verify_construct_inventory(pair, CONSTRUCTS, [count; 6])?;
    super::verify_construct_inventory(pair, [result_maps::RESULT], [count])?;
    if pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.annotations.contains_key(KEY))
        .count()
        != count * 6
    {
        return Err(
            "Constructor Result map annotation roster disagrees with checked source".into(),
        );
    }

    let Some(checked) = checked else {
        return Ok(());
    };
    let guards = super::conditional_payloads::Guards::new(pair)?;
    for stage in &checked.stages {
        invocation(pair, stage, &guards)?;
    }

    whole_return(pair, &checked.stages[1], facts)
}

fn operations<'a>(pair: PairView<'a>, checked: &CheckedMap<'_>) -> Result<Operations<'a>, String> {
    let payloads = checked.payload_types().map(crate::data_types::token);
    let output = crate::data_types::token(&checked.output)?;
    Ok(Operations {
        saved: receiver_value(pair, checked)?,
        selector: typed_operation(pair, checked, CONSTRUCTS[0], "bool")?,
        payloads: [
            typed_operation(pair, checked, CONSTRUCTS[1], &payloads[0].clone()?)?,
            typed_operation(pair, checked, CONSTRUCTS[2], &payloads[1].clone()?)?,
        ],
        constructor: typed_operation(
            pair,
            checked,
            CONSTRUCTS[3],
            &crate::data_types::token(checked.constructed_type())?,
        )?,
        results: [
            typed_operation(pair, checked, CONSTRUCTS[4], &output)?,
            typed_operation(pair, checked, CONSTRUCTS[5], &output)?,
        ],
        merge: map_value(pair, checked)?,
    })
}

fn invocation(
    pair: PairView<'_>,
    checked: &CheckedMap<'_>,
    guards: &super::conditional_payloads::Guards<'_>,
) -> Result<(), String> {
    let operations = operations(pair, checked)?;
    effects(checked, &operations)?;
    operands(pair, checked, &operations)?;
    branches(guards, checked, &operations)?;
    order(pair, checked, &operations)?;

    merge(pair, &operations)
}

fn role(node: &PdgNode) -> Option<&str> {
    node.attributes
        .extensions
        .get("compat.rust.hir-r0:source-operation")?
        .get("role")?
        .as_str()
}

fn tagged_operation<'a>(
    pair: PairView<'a>,
    source: &crate::HirNode,
    stage: &str,
    construct: &str,
) -> Result<&'a PdgNode, String> {
    let mut candidates = pair.pdg.nodes.iter().filter(|node| {
        role(node) == Some(stage)
            && node.attributes.construct.as_deref() == Some(construct)
            && super::aggregates::source_span(node).is_ok_and(|span| {
                span.start == u64::from(source.span.start) && span.end == u64::from(source.span.end)
            })
    });
    let node = candidates
        .next()
        .ok_or("Missing exact Result map stage operation")?;
    if candidates.next().is_some() {
        return Err("Ambiguous exact Result map stage operation".into());
    }
    Ok(node)
}

fn typed_operation<'a>(
    pair: PairView<'a>,
    checked: &CheckedMap<'_>,
    construct: &str,
    ty: &str,
) -> Result<&'a PdgNode, String> {
    let source = if construct == CONSTRUCTS[3] {
        checked.callback
    } else {
        checked.source
    };
    let node = tagged_operation(pair, source, checked.role(), construct)?;
    if node.original_kind.known_value() != Some(source.kind.as_str())
        || node
            .annotations
            .get("compat.rust.hir-r0:type")
            .and_then(|value| value.get("value"))
            .and_then(serde_json::Value::as_str)
            != Some(ty)
    {
        return Err("Result map stage loses its exact source kind or value type".into());
    }
    Ok(node)
}

fn receiver_value<'a>(pair: PairView<'a>, checked: &CheckedMap<'_>) -> Result<&'a PdgNode, String> {
    if !checked.maps_error {
        return super::value_operation(pair, checked.receiver);
    }
    let value = tagged_operation(
        pair,
        checked.receiver,
        "result-map-success-stage",
        result_maps::RESULT,
    )?;
    super::aggregates::check_value(value, checked.receiver)?;
    Ok(value)
}

fn map_value<'a>(pair: PairView<'a>, checked: &CheckedMap<'_>) -> Result<&'a PdgNode, String> {
    let value = tagged_operation(pair, checked.source, checked.role(), result_maps::RESULT)?;
    if value.category != NodeCategory::Merge {
        return Err("Transport Result lacks its dedicated conditional merge".into());
    }
    super::aggregates::check_value(value, checked.source)?;
    Ok(value)
}

fn effects(checked: &CheckedMap<'_>, operations: &Operations<'_>) -> Result<(), String> {
    let metadata = metadata(checked)?;
    for operation in [
        operations.selector,
        operations.payloads[0],
        operations.payloads[1],
        operations.constructor,
        operations.results[0],
        operations.results[1],
    ] {
        let category = if operation.id == operations.selector.id {
            NodeCategory::Control
        } else {
            NodeCategory::Computation
        };
        super::callbacks::effects(operation, category)?;
        if role(operation) != Some(checked.role())
            || operation.annotations.get(KEY) != Some(&metadata)
        {
            return Err(
                "Constructor Result operation loses its exact checked interpretation".into(),
            );
        }
    }
    if role(operations.merge) != Some(checked.role()) {
        return Err("Result map merge changes its exact checked stage".into());
    }
    super::callbacks::effects(operations.merge, NodeCategory::Merge)
}

fn operands(
    pair: PairView<'_>,
    checked: &CheckedMap<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    for operation in [ops.selector, ops.payloads[0], ops.payloads[1]] {
        super::verify_saved_operands(pair, operation, &[ops.saved])?;
    }
    consumers(
        pair,
        ops.saved,
        &[ops.selector, ops.payloads[0], ops.payloads[1]],
        "value_operand",
    )?;

    let selected = usize::from(checked.maps_error);
    super::verify_saved_operands(pair, ops.constructor, &[ops.payloads[selected]])?;
    for index in 0..2 {
        let value = if index == selected {
            ops.constructor
        } else {
            ops.payloads[index]
        };
        super::verify_saved_operands(pair, ops.results[index], &[value])?;
        let consumer = if index == selected {
            ops.constructor
        } else {
            ops.results[index]
        };
        consumers(pair, ops.payloads[index], &[consumer], "value_operand")?;
    }

    consumers(
        pair,
        ops.constructor,
        &[ops.results[selected]],
        "value_operand",
    )
}

fn order(pair: PairView<'_>, checked: &CheckedMap<'_>, ops: &Operations<'_>) -> Result<(), String> {
    super::callbacks::order::before(
        pair,
        &super::callbacks::order::completion(pair, ops.saved)?,
        ops.selector,
    )?;
    for index in 0..2 {
        let selected = index == usize::from(checked.maps_error);
        let consumer = if selected {
            ops.constructor
        } else {
            ops.results[index]
        };
        super::callbacks::order::before(
            pair,
            &super::callbacks::order::completion(pair, ops.payloads[index])?,
            consumer,
        )?;
    }
    super::callbacks::order::before(
        pair,
        &super::callbacks::order::completion(pair, ops.constructor)?,
        ops.results[usize::from(checked.maps_error)],
    )
}

fn branches(
    guards: &super::conditional_payloads::Guards<'_>,
    checked: &CheckedMap<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    for (index, outcome) in [CfgEdgeKind::False, CfgEdgeKind::True]
        .into_iter()
        .enumerate()
    {
        guards.guarded(ops.selector, ops.payloads[index], outcome)?;
        guards.guarded(ops.selector, ops.results[index], outcome)?;
    }
    guards.guarded(
        ops.selector,
        ops.constructor,
        if checked.maps_error {
            CfgEdgeKind::True
        } else {
            CfgEdgeKind::False
        },
    )
}

fn merge(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    super::verify_saved_operands(pair, ops.merge, &[])?;

    let incoming: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| edge.target == ops.merge.id && edge.kind == crate::EdgeFamily::Data)
        .collect();
    if incoming.len() != 2
        || incoming.iter().any(|edge| {
            edge.subkind != "value_alternative"
                || !ops.results.iter().any(|value| value.id == edge.source)
        })
    {
        return Err("Result map merge invents an unrelated payload alternative".into());
    }

    let site = super::callbacks::order::site(pair, &ops.merge.id)?;
    let alternatives = site["alternatives"]
        .as_array()
        .ok_or("Missing Result map merge alternatives")?;
    if ops.merge.category != NodeCategory::Merge
        || alternatives.len() != 2
        || site["controller_pdg_node_id"] != ops.selector.id
    {
        return Err("Result map merge loses its exact discriminant controller".into());
    }
    for (index, outcome) in ["false", "true"].into_iter().enumerate() {
        if alternatives
            .iter()
            .filter(|alternative| {
                alternative["outcome"] == outcome
                    && alternative["value_pdg_node_id"] == ops.results[index].id
            })
            .count()
            != 1
        {
            return Err("Result map merge changes or forwards a branch payload".into());
        }
        consumers(pair, ops.results[index], &[ops.merge], "value_alternative")?;
    }

    Ok(())
}

fn whole_return(
    pair: PairView<'_>,
    stage: &CheckedMap<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let result = map_value(pair, stage)?;
    let returned = super::serialized_operation(pair, &stage.source.span, |node| {
        node.category == NodeCategory::Return
    })?;
    super::verify_saved_operands(pair, returned, &[result])?;
    consumers(pair, result, &[returned], "value_operand")?;
    super::verify_value_effects(returned, false, "Result transport return invents effects")?;

    if facts.signature.return_ty.normalized.as_deref()
        != Some(crate::data_types::token(&stage.output)?.as_str())
        || returned
            .attributes
            .extensions
            .get("compat.rust.hir-r0:return-form")
            .and_then(serde_json::Value::as_str)
            != Some("implicit")
    {
        return Err("Transport must return the complete final Result once".into());
    }

    super::result_propagation::return_to_exit(pair, returned)
}
