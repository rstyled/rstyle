//! Binding dataflow, dominance, and one-exit postdominance on normal flow.

use super::{Graph, Operation};
use crate::{EdgeFamily, NodeCategory};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type DefinitionSet = BTreeMap<String, BTreeSet<String>>;
#[cfg(test)]
type Sets = BTreeMap<String, BTreeSet<String>>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependence {
    pub source: String,
    pub target: String,
    pub family: EdgeFamily,
    pub subkind: String,
    pub position: Option<u64>,
    pub definition: Option<String>,
    pub symbol: Option<String>,
    pub outcome: Option<String>,
    pub guards: Option<super::Guard>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Analysis {
    pub edges: Vec<Dependence>,
    pub reachable: BTreeSet<String>,
    pub definition_in: BTreeMap<String, DefinitionSet>,
    pub definition_out: BTreeMap<String, DefinitionSet>,
}

pub fn analyze(graph: &Graph) -> Result<Analysis, String> {
    analyze_policy(graph, false)
}

pub fn analyze_cyclic(graph: &Graph) -> Result<Analysis, String> {
    analyze_policy(graph, true)
}

fn analyze_policy(graph: &Graph, cyclic: bool) -> Result<Analysis, String> {
    indexed::analyze(graph, cyclic)
}

pub fn reachability(graph: &Graph) -> BTreeSet<String> {
    topology::reachable(graph).into_iter().collect()
}

fn edge(source: &str, target: &str, family: EdgeFamily, subkind: &str) -> Dependence {
    Dependence {
        source: source.to_string(),
        target: target.to_string(),
        family,
        subkind: subkind.to_string(),
        position: None,
        definition: None,
        symbol: None,
        outcome: None,
        guards: None,
    }
}

fn value_edges<'a>(nodes: impl Iterator<Item = &'a Operation>) -> impl Iterator<Item = Dependence> {
    nodes.flat_map(|node| {
        node.operands
            .iter()
            .enumerate()
            .map(move |(position, value)| Dependence {
                position: Some(position as u64),
                ..edge(value, &node.id, EdgeFamily::Data, "value_operand")
            })
    })
}

fn alternative_edges(graph: &Graph, reached: impl Fn(&str) -> bool) -> Vec<Dependence> {
    let alternatives = graph
        .value_sites
        .iter()
        .filter(|s| {
            s.cfg_successor_node_id
                .as_ref()
                .is_some_and(|id| reached(id))
        })
        .flat_map(|site| {
            site.alternatives.iter().map(move |a| Dependence {
                position: if let Some(index) = a.outcome.strip_prefix("arm:") {
                    index.parse().ok()
                } else {
                    Some(if a.outcome == "true" { 0 } else { 1 })
                },
                outcome: Some(a.outcome.clone()),
                guards: Some(a.guards.clone()),
                ..edge(
                    &a.value_pdg_node_id,
                    &site.pdg_node_id,
                    EdgeFamily::Data,
                    "value_alternative",
                )
            })
        });

    alternatives.collect()
}

mod bits;
mod dominance;
mod flow;
mod indexed;
mod post;
mod topology;
use post::{Admission, PostTree};
use topology::Topology;

#[cfg(test)]
mod reference;
#[cfg(test)]
use reference::*;
#[cfg(test)]
#[path = "analysis_tests.rs"]
mod tests;
