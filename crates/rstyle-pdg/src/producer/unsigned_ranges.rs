//! Ephemeral unsigned upper bounds over the existing reaching-definition graph.
//! Missing inputs use the complete resolved type range, never zero.
use super::{Analysis, Graph, Operation};
use crate::NodeCategory;
use std::collections::BTreeMap;

pub(super) const CAPABILITY: &str = "rust.bounded-unsigned-reads";
pub(super) const METHOD: &str = "resolved unsigned type bounds, reaching-definition snapshots, constant division/remainder and lossless conversion";
pub(super) const KEY: &str = "compat.rust.hir-r0:exceptional-flow-assessment";

pub(super) fn needed(facts: &dyn crate::unit_store::EvidenceFacts, graph: &Graph) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && graph.nodes.iter().any(|node| {
            node.construct.as_deref() == Some("data.value-index")
                || matches!(node.operator.as_deref(), Some("<<" | ">>"))
        })
}

pub(super) struct Ranges<'a> {
    graph: &'a Graph,
    analysis: &'a Analysis,
    width: Option<u32>,
    bounds: BTreeMap<String, u128>,
}

impl<'a> Ranges<'a> {
    pub(super) fn new(
        facts: &dyn crate::unit_store::EvidenceFacts,
        graph: &'a Graph,
        analysis: &'a Analysis,
    ) -> Self {
        Self {
            graph,
            analysis,
            width: facts
                .build
                .pointer_width
                .value
                .and_then(|width| u32::try_from(width).ok()),
            bounds: BTreeMap::new(),
        }
    }

    pub(super) fn observe(&mut self, node: &Operation, constants: &BTreeMap<String, Option<i128>>) {
        if let Some(bound) = self.maximum(node, constants) {
            self.bounds.insert(node.id.clone(), bound);
        }
    }

    fn maximum(
        &self,
        node: &Operation,
        constants: &BTreeMap<String, Option<i128>>,
    ) -> Option<u128> {
        let full = type_maximum(&node.ty, self.width)?;
        if let Some(value) = constants.get(&node.id).copied().flatten() {
            return u128::try_from(value).ok().filter(|value| *value <= full);
        }
        let inferred = if node.category == NodeCategory::Read {
            read(node, self.graph, self.analysis, &self.bounds)
        } else {
            expression(node, constants, &self.bounds)
        };
        Some(inferred.unwrap_or(full).min(full))
    }

    pub(super) fn safe(&self, node: &Operation) -> bool {
        safe(node, self.graph, &self.bounds, self.width)
    }
}

fn type_maximum(ty: &str, width: Option<u32>) -> Option<u128> {
    let bits = match ty {
        "u8" => 8,
        "u16" => 16,
        "u32" => 32,
        "u64" => 64,
        "u128" => 128,
        "usize" => width?,
        _ => return None,
    };
    match bits {
        1..=127 => Some((1u128 << bits) - 1),
        128 => Some(u128::MAX),
        _ => None,
    }
}

fn read(
    node: &Operation,
    graph: &Graph,
    analysis: &Analysis,
    bounds: &BTreeMap<String, u128>,
) -> Option<u128> {
    let definitions = analysis
        .definition_in
        .get(&node.id)?
        .get(node.symbol.as_ref()?)?;
    let values: Option<Vec<_>> = definitions
        .iter()
        .map(|id| {
            let definition = graph
                .nodes
                .iter()
                .find(|node| node.definition.as_ref() == Some(id))?;
            bounds.get(definition.operands.first()?).copied()
        })
        .collect();
    values?.into_iter().max()
}

fn expression(
    node: &Operation,
    constants: &BTreeMap<String, Option<i128>>,
    bounds: &BTreeMap<String, u128>,
) -> Option<u128> {
    let first = *bounds.get(node.operands.first()?)?;
    if node.construct.as_deref() == Some("data.scalar-from") {
        return Some(first);
    }

    let denominator = constants.get(node.operands.get(1)?)?.as_ref()?;
    let denominator = u128::try_from(*denominator)
        .ok()
        .filter(|value| *value != 0)?;
    match node.operator.as_deref()? {
        "/" => Some(first / denominator),
        "%" => Some(first.min(denominator - 1)),
        _ => None,
    }
}

pub(super) fn safe(
    node: &Operation,
    graph: &Graph,
    bounds: &BTreeMap<String, u128>,
    width: Option<u32>,
) -> bool {
    let Some(index) = node.operands.get(1).and_then(|id| bounds.get(id)).copied() else {
        return false;
    };
    if matches!(node.operator.as_deref(), Some("<<" | ">>")) {
        let bits = if node.ty == "usize" || node.ty == "isize" {
            width
        } else {
            node.ty.get(1..).and_then(|ty| ty.parse::<u32>().ok())
        };
        return bits.is_some_and(|bits| index < u128::from(bits));
    }
    if node.construct.as_deref() != Some("data.value-index") {
        return false;
    }

    let base = graph
        .nodes
        .iter()
        .find(|base| node.operands.first() == Some(&base.id));
    base.and_then(|base| crate::data_types::parse(&base.ty).ok()).is_some_and(|ty| {
        matches!(ty, crate::data_types::TypeToken::Array { length, .. } if index < u128::from(length))
    })
}

pub(crate) fn bounded_value_index(
    pair: super::PairView<'_>,
    node: &crate::PdgNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && crate::extraction_profile::policy_supports(&pair.pdg.profile, CAPABILITY)
        && node.attributes.construct.as_deref() == Some("data.value-index")
        && proofs(pair).is_some_and(|proofs| {
            proofs.iter().any(|proof| {
                proof
                    .get("operation_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(node.id.as_str())
                    && proof.get("method").and_then(serde_json::Value::as_str) == Some(METHOD)
                    && proof
                        .get("omitted_panic_impossible")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true)
            })
        })
}

pub(crate) fn verify_range_policy(pair: super::PairView<'_>) -> Result<(), String> {
    if crate::extraction_profile::policy_supports(&pair.pdg.profile, CAPABILITY) {
        return Ok(());
    }
    if proofs(pair).is_some_and(|proofs| {
        proofs
            .iter()
            .any(|proof| proof.get("method").and_then(serde_json::Value::as_str) == Some(METHOD))
    }) {
        return Err("Unsigned range proofs require their registered document policy".into());
    }
    Ok(())
}

fn proofs<'a>(pair: super::PairView<'a>) -> Option<&'a Vec<serde_json::Value>> {
    pair.pdg
        .extensions
        .get(KEY)?
        .get("checked_operations")?
        .as_array()
}
