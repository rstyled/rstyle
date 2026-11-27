//! Relate address and closure construction sites to loop binding lifetimes. The
//! producer declares how a language renews a binding. The CFG determines which
//! sites can actually repeat via the corresponding loop's backedge.

use super::{Flow, Tree, Vertex};
use crate::cpg::model::{Function, Node, NodeKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IterationStorage {
    pub loop_node: String,
    pub symbol: String,
    pub site: String,
    pub distinct_across_iterations: bool,
}

#[derive(Deserialize)]
struct LoopBindings<'a> {
    head: &'a str,
    bindings: BTreeMap<&'a str, &'a str>,
}

fn members(flow: &Flow<'_>, dom: &Tree, head: Vertex, tails: &[Vertex]) -> HashSet<Vertex> {
    let mut seen = HashSet::from([head]);
    let mut pending = tails.to_vec();
    while let Some(node) = pending.pop() {
        if dom.contains(head, node) && seen.insert(node) {
            pending.extend(&flow.forward.predecessors[node]);
        }
    }
    seen
}

fn uses_storage(node: &Node, symbol: &str, accesses: &HashMap<&str, &str>) -> bool {
    match node.kind {
        NodeKind::Closure => node.captures.iter().flatten().any(|c| c.symbol == symbol),
        NodeKind::Reference => node
            .children
            .first()
            .is_some_and(|child| accesses.get(child.as_str()) == Some(&symbol)),
        _ => false,
    }
}

fn bindings(function: &Function) -> Option<BTreeMap<&str, LoopBindings<'_>>> {
    let facts = function
        .extensions
        .as_ref()
        .and_then(|e| e.get("go:iteration-bindings"))?;
    BTreeMap::deserialize(facts).ok()
}

pub(super) fn derive(function: &Function, flow: &Flow<'_>, dom: &Tree) -> Vec<IterationStorage> {
    let Some(facts) = bindings(function) else {
        return Vec::new();
    };
    let accesses = function
        .accesses
        .iter()
        .map(|a| (a.node.as_str(), a.symbol.as_str()))
        .collect();
    let mut tails: HashMap<Vertex, Vec<Vertex>> = HashMap::new();
    for (tail, head) in flow.back_edges() {
        tails.entry(head).or_default().push(tail);
    }
    let sites: Vec<_> = function
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Closure | NodeKind::Reference))
        .filter_map(|node| Some((flow.vertex(&node.id)?, node)))
        .collect();
    let mut out = Vec::new();
    for (loop_node, fact) in facts {
        let Some(head) = flow.vertex(fact.head) else {
            continue;
        };
        let Some(tails) = tails.get(&head) else {
            continue;
        };
        let natural_loop = members(flow, dom, head, tails);
        for (symbol, policy) in fact.bindings {
            let distinct = match policy {
                "reuse" => false,
                "copy_before_post" | "range_item" => true,
                _ => continue,
            };
            out.extend(
                sites
                    .iter()
                    .filter(|(v, n)| natural_loop.contains(v) && uses_storage(n, symbol, &accesses))
                    .map(|(_, node)| IterationStorage {
                        loop_node: loop_node.to_string(),
                        symbol: symbol.to_string(),
                        site: node.id.clone(),
                        distinct_across_iterations: distinct,
                    }),
            );
        }
    }
    out
}
