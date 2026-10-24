//! Exact borrowed operation keys; lookup never grants semantic authority.
use super::*;
use std::collections::{BTreeSet, HashMap};

#[derive(PartialEq, Eq, Hash)]
struct OperationKey<'a> {
    category: std::mem::Discriminant<NodeCategory>,
    construct: Option<&'a str>,
    span: Option<(u64, u64)>,
    kind_status: std::mem::Discriminant<FactStatus>,
    kind_value: Option<&'a str>,
    kind_reason: Option<&'a str>,
    position: Option<u64>,
    source_operation: Option<&'a Value>,
}

impl<'a> OperationKey<'a> {
    fn of(node: &'a PdgNode) -> Self {
        Self {
            category: std::mem::discriminant(&node.category),
            construct: node.attributes.construct.as_deref(),
            span: node.source_span.as_ref().map(|span| (span.start, span.end)),
            kind_status: std::mem::discriminant(&node.original_kind.status),
            kind_value: node.original_kind.value.as_deref(),
            kind_reason: node.original_kind.reason.as_deref(),
            position: node.attributes.position,
            source_operation: node
                .attributes
                .extensions
                .get("compat.rust.hir-r0:source-operation"),
        }
    }
}

pub(super) fn node_names(
    actual: PairView<'_>,
    expected: PairView<'_>,
) -> Result<BTreeMap<String, String>, Vec<String>> {
    if actual.pdg.nodes.len() != expected.pdg.nodes.len() {
        return Err(vec!["Source/value operation inventory differs".to_string()]);
    }
    let index = expected_index(&expected.pdg.nodes);
    let mut names = BTreeMap::new();
    let mut claimed = BTreeSet::new();
    for node in &actual.pdg.nodes {
        let Some(Some(candidate)) = index.get(&OperationKey::of(node)) else {
            return Err(vec![format!(
                "No unique source-backed correspondence for node {}",
                node.id
            )]);
        };
        if !claimed.insert(candidate.id.as_str()) {
            return Err(vec![
                "Two nodes claim the same source operation".to_string(),
            ]);
        }
        if let Some(previous) = names.insert(node.id.clone(), candidate.id.clone()) {
            claimed.remove(previous.as_str());
        }
    }

    Ok(names)
}

fn expected_index(nodes: &[PdgNode]) -> HashMap<OperationKey<'_>, Option<&PdgNode>> {
    let mut index = HashMap::with_capacity(nodes.len());
    for node in nodes {
        index
            .entry(OperationKey::of(node))
            .and_modify(|value| *value = None)
            .or_insert(Some(node));
    }

    index
}
