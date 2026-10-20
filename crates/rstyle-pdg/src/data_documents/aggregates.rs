//! Independently check serialized constructor slots against original operands.

use crate::{HirNode, NodeCategory, PdgNode, producer::PairView, unit_store::EvidenceFacts};
use std::collections::BTreeMap;

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let mut constructors = Vec::new();
    collect(&facts.body, &mut constructors, facts)?;
    if constructors.is_empty() {
        return Ok(());
    }

    let context = Context::new(pair);
    for source in constructors {
        context.verify_constructor(source, facts)?;
    }
    Ok(())
}

type SourceKey<'a> = (u64, u64, Option<&'a str>);

struct Context<'a> {
    pair: PairView<'a>,
    nodes: BTreeMap<&'a str, &'a PdgNode>,
    sources: BTreeMap<SourceKey<'a>, Vec<&'a PdgNode>>,
    operands: BTreeMap<&'a str, Vec<&'a crate::PdgEdge>>,
}

impl<'a> Context<'a> {
    fn new(pair: PairView<'a>) -> Self {
        let nodes = pair
            .pdg
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        let sources = pair
            .pdg
            .nodes
            .iter()
            .filter_map(|node| {
                let span = node.source_span.as_ref()?;
                (node.category == NodeCategory::Computation).then_some((
                    (span.start, span.end, node.original_kind.known_value()),
                    node,
                ))
            })
            .fold(BTreeMap::<_, Vec<_>>::new(), |mut map, (key, node)| {
                map.entry(key).or_default().push(node);
                map
            });
        let operands = pair
            .pdg
            .edges
            .iter()
            .filter(|edge| edge.kind == crate::EdgeFamily::Data && edge.subkind == "value_operand")
            .fold(BTreeMap::<_, Vec<_>>::new(), |mut map, edge| {
                map.entry(edge.target.as_str()).or_default().push(edge);
                map
            });

        Self {
            pair,
            nodes,
            sources,
            operands,
        }
    }

    fn verify_constructor(
        &self,
        source: &HirNode,
        facts: &dyn EvidenceFacts,
    ) -> Result<(), String> {
        let candidates = self
            .sources
            .get(&(
                u64::from(source.span.start),
                u64::from(source.span.end),
                Some(source.kind.as_str()),
            ))
            .ok_or("Missing serialized aggregate constructor")?;
        let [node] = candidates.as_slice() else {
            return Err("Ambiguous serialized aggregate constructor".into());
        };
        verify_semantics(node, source, facts)?;
        let edges = self
            .operands
            .get(node.id.as_str())
            .map(Vec::as_slice)
            .unwrap_or_default();
        if edges.len() != source.children.len() {
            if !super::live_operations(self.pair)?.contains(node.id.as_str()) {
                return Ok(());
            }
            return Err("Aggregate operand slot count disagrees".into());
        }

        let slots: BTreeMap<_, _> = edges
            .iter()
            .map(|edge| (edge.operand_position, *edge))
            .collect();
        if slots.len() != edges.len() {
            return Err("Duplicate aggregate operand position".into());
        }
        for (position, child) in source.children.iter().enumerate() {
            let edge = slots
                .get(&Some(position as u64))
                .ok_or("Missing ordered aggregate component")?;
            let value = self
                .nodes
                .get(edge.source.as_str())
                .ok_or("Missing aggregate component value")?;
            check_value(value, child)?;
        }
        Ok(())
    }
}

fn collect<'a>(
    node: &'a HirNode,
    constructors: &mut Vec<&'a HirNode>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if crate::data_subslices::candidate(node, facts) {
        return collect_subslice(node, constructors, facts);
    }
    let children = if node.kind == "expr_tuple_assign" {
        crate::tuple_assignment::verify(node, facts)?;
        &node.children[1..]
    } else {
        node.children.as_slice()
    };

    if crate::data_aggregates::constructor(node) {
        constructors.push(node);
    }
    for child in children {
        collect(child, constructors, facts)?;
    }
    Ok(())
}

fn collect_subslice<'a>(
    node: &'a HirNode,
    constructors: &mut Vec<&'a HirNode>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let checked = crate::data_subslices::verify(node, facts)?;
    for operand in checked.operands() {
        collect(operand, constructors, facts)?;
    }
    Ok(())
}

pub(super) fn check_value(value: &PdgNode, mut child: &HirNode) -> Result<(), String> {
    let token = child
        .ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        .ok_or("Missing aggregate component type")?;
    if super::origins::wire_type(value)? != token {
        return Err("Serialized aggregate component type disagrees".into());
    }

    if value.attributes.construct.as_deref() != Some("unit.value") {
        while child.kind == "expr_block"
            && let Some(tail) = child.children.last()
            && tail.kind.starts_with("expr_")
        {
            child = tail;
        }
    } else if token != "unit"
        || value
            .attributes
            .literal
            .as_ref()
            .and_then(|fact| fact.known_value())
            != Some("()")
    {
        return Err("Invented aggregate unit component".into());
    }
    let span = source_span(value)?;

    if span.start != u64::from(child.span.start)
        || span.end != u64::from(child.span.end)
        || (value.category == NodeCategory::Merge
            && !matches!(child.kind.as_str(), "expr_if" | "expr_match"))
        || (value.category != NodeCategory::Merge
            && value.original_kind.known_value() != Some(child.kind.as_str()))
    {
        return Err("Aggregate operand loses its original component position/value".into());
    }
    Ok(())
}

pub(super) fn source_span(value: &PdgNode) -> Result<crate::Span, String> {
    if let Some(span) = &value.source_span {
        return Ok(span.clone());
    }
    serde_json::from_value(
        value
            .attributes
            .extensions
            .get("compat.rust.hir-r0:source-operation")
            .and_then(|entry| entry.get("construct_span"))
            .ok_or("Missing aggregate operand source span")?
            .clone(),
    )
    .map_err(|error| error.to_string())
}

fn verify_structure(
    node: &PdgNode,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let expected = crate::data_structs::descriptor(source, facts)?;
    if node.attributes.construct.as_deref() != Some("data.struct")
        || node
            .attributes
            .literal
            .as_ref()
            .and_then(|literal| literal.known_value())
            != Some(expected.as_str())
    {
        return Err("Serialized struct loses exact nominal field slot mapping".into());
    }
    Ok(())
}

fn verify_semantics(
    node: &PdgNode,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    super::verify_value_effects(
        node,
        source.kind == "expr_struct",
        "Aggregate construction invents scalar semantics or memory effects",
    )?;
    if source.kind == "expr_struct" {
        verify_structure(node, source, facts)?;
    }
    Ok(())
}
