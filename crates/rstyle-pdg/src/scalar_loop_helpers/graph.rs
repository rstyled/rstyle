//! Bind the source rank to the complete independently checked cyclic graph.
use super::source::Shape;
use crate::{ByteSpan, NodeCategory, PdgNode, producer::PairView, unit_store::EvidenceFacts};

pub(super) fn verify(
    facts: &dyn EvidenceFacts,
    pair: PairView<'_>,
    source: &Shape,
) -> Result<(), String> {
    preflight(facts, pair)?;

    let header = construct(pair, "loop", source.loop_span)?;
    let exit = construct(pair, "break", source.break_span)?;
    let division = operation(pair, NodeCategory::Computation, "/", source.divide)?;
    let remainder = operation(pair, NodeCategory::Computation, "%", source.remainder)?;
    let value_update = operation(pair, NodeCategory::Assignment, "=", source.value_update)?;
    let count_update = operation(pair, NodeCategory::Assignment, "+=", source.count_update)?;
    let assertions: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some("core::debug_assert"))
        .collect();
    let [assertion] = assertions.as_slice() else {
        return Err("Bounded scalar helper requires its one active builtin assertion".into());
    };
    flow(pair, header, exit, count_update)?;

    let dominators = crate::cyclic_values::dominators(pair.cfg);
    for (before, after) in [
        (header, *assertion),
        (*assertion, division),
        (division, remainder),
        (remainder, exit),
        (remainder, value_update),
        (value_update, count_update),
    ] {
        if !dominators
            .get(&after.id)
            .is_some_and(|set| set.contains(&before.id))
        {
            return Err(
                "Bounded scalar helper changes assertion/division/exit/update order".into(),
            );
        }
    }
    Ok(())
}

fn preflight(facts: &dyn EvidenceFacts, pair: PairView<'_>) -> Result<(), String> {
    // Source grammar contains no helper invocations. Verifying it cannot recurse
    // through the helper boundary currently being established.
    crate::producer::verify_view(pair, facts).map_err(|errors| errors.join("; "))?;
    crate::cyclic_values::validate(pair.pdg, pair.cfg).map_err(|errors| errors.join("; "))?;
    if pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Partial
        || !pair.pdg.memory_locations.is_empty()
        || pair
            .pdg
            .nodes
            .iter()
            .any(|node| node.category == NodeCategory::Call)
    {
        return Err(
            "Bounded scalar helper must retain its assertion limitation and scalar effects".into(),
        );
    }
    Ok(())
}

fn construct<'a>(
    pair: PairView<'a>,
    name: &str,
    expected: ByteSpan,
) -> Result<&'a PdgNode, String> {
    let candidates: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some(name))
        .collect();
    let [node] = candidates.as_slice() else {
        return Err(format!(
            "Bounded scalar helper requires exactly one {name} transfer"
        ));
    };
    check_span(
        node.attributes
            .extensions
            .get("compat.rust.hir-r0:source-operation")
            .ok_or("Bounded scalar helper graph lacks its source anchor")?,
        expected,
    )?;
    Ok(node)
}

fn operation<'a>(
    pair: PairView<'a>,
    category: NodeCategory,
    operator: &str,
    expected: ByteSpan,
) -> Result<&'a PdgNode, String> {
    let candidates: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.category == category
                && node
                    .attributes
                    .operator
                    .as_ref()
                    .and_then(|fact| fact.known_value())
                    == Some(operator)
                && node
                    .attributes
                    .extensions
                    .get("compat.rust.hir-r0:source-operation")
                    .is_some_and(|anchor| check_span(anchor, expected).is_ok())
        })
        .collect();
    match candidates.as_slice() {
        [node] => Ok(node),
        _ => Err("Bounded scalar helper graph loses its exact source operation".into()),
    }
}

fn check_span(anchor: &serde_json::Value, expected: ByteSpan) -> Result<(), String> {
    let span: crate::Span = serde_json::from_value(anchor["construct_span"].clone())
        .map_err(|_| "Bounded scalar helper graph lacks a source anchor")?;
    if span.source_id != "source"
        || span.start != u64::from(expected.start)
        || span.end != u64::from(expected.end)
    {
        return Err("Bounded scalar helper graph/source operation mismatch".into());
    }
    Ok(())
}

fn flow(
    pair: PairView<'_>,
    header: &PdgNode,
    exit: &PdgNode,
    count_update: &PdgNode,
) -> Result<(), String> {
    use crate::cfg::{CfgEdgeKind, LoopRole};
    let backedges: Vec<_> = pair
        .cfg
        .edges
        .iter()
        .filter(|edge| edge.loop_role == Some(LoopRole::BackEdge))
        .collect();
    let exits: Vec<_> = pair
        .cfg
        .edges
        .iter()
        .filter(|edge| edge.loop_role == Some(LoopRole::LoopExit))
        .collect();

    if !matches!(backedges.as_slice(), [edge] if edge.source == count_update.id && edge.target == header.id)
        || !matches!(exits.as_slice(), [edge] if edge.source == exit.id && edge.kind == CfgEdgeKind::Break)
        || pair.pdg.nodes.iter().any(|node| {
            matches!(
                node.attributes.construct.as_deref(),
                Some("while" | "continue")
            )
        })
    {
        return Err("Bounded scalar helper changes its only backedge or break exit".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graph_anchor_requires_exact_original_source_and_range() {
        let expected = ByteSpan { start: 12, end: 19 };
        let anchor =
            serde_json::json!({"construct_span":{"source_id":"source","start":12,"end":19}});
        assert!(check_span(&anchor, expected).is_ok());
        for changed in [
            serde_json::json!({"construct_span":{"source_id":"other","start":12,"end":19}}),
            serde_json::json!({"construct_span":{"source_id":"source","start":11,"end":19}}),
            serde_json::json!({"construct_span":{"source_id":"source","start":12,"end":20}}),
            serde_json::json!({"construct_span":null}),
        ] {
            assert!(check_span(&changed, expected).is_err());
        }
    }
}
