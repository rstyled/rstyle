//! Relate address and closure construction sites to loop binding lifetimes. The
//! producer declares how a language renews a binding. The CFG determines which
//! sites can actually repeat via the corresponding loop's backedge.

use super::{Flow, Tree, Vertex};
use crate::cpg::model::{AccessMode, Function, Node, NodeKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IterationStorage {
    pub loop_node: String,
    pub symbol: String,
    pub site: String,
    pub distinct_across_iterations: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IterationPhase {
    pub loop_node: String,
    pub symbol: String,
    pub first_declaration: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy_before_post: Option<String>,
    /// Direct writes which can supply the previous iteration's value at the copy.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub last_direct_writes: BTreeSet<String>,
}

#[derive(Deserialize)]
struct LoopBindings<'a> {
    head: &'a str,
    #[serde(default)]
    initial: BTreeMap<&'a str, &'a str>,
    post_entry: Option<&'a str>,
    bindings: BTreeMap<&'a str, &'a str>,
}

pub(super) fn members(
    flow: &Flow<'_>,
    dom: &Tree,
    head: Vertex,
    tails: &[Vertex],
) -> HashSet<Vertex> {
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

fn writes_before(flow: &Flow<'_>, post: Vertex, writes: &HashSet<Vertex>) -> BTreeSet<String> {
    let mut pending = flow.forward.predecessors[post].clone();
    let mut seen = HashSet::new();
    let mut out = BTreeSet::new();
    while let Some(v) = pending.pop() {
        if !seen.insert(v) {
            continue;
        }
        if writes.contains(&v) {
            out.insert(flow.id(v).to_owned());
        } else {
            pending.extend(&flow.forward.predecessors[v]);
        }
    }
    out
}

struct PhaseFrame<'a, 'f> {
    function: &'a Function,
    graph: &'a Flow<'f>,
    dom: &'a Tree,
}

impl PhaseFrame<'_, '_> {
    fn phase(
        &self,
        info: (&str, &str, &LoopBindings<'_>),
        policy: &str,
        natural: &HashSet<Vertex>,
    ) -> Option<IterationPhase> {
        let (owner, symbol, b) = info;
        let source = b.initial.get(symbol)?;
        let f = self.graph;
        let init = f.vertex(source)?;
        if !self.dom.contains(init, f.vertex(b.head)?) || natural.contains(&init) {
            return None;
        }
        let writes: HashSet<_> = self
            .function
            .accesses
            .iter()
            .filter(|a| a.symbol == symbol && a.mode != AccessMode::Read)
            .filter_map(|a| f.vertex(&a.node))
            .collect();
        if !writes.contains(&init) {
            return None;
        }
        let post = b
            .post_entry
            .and_then(|id| f.vertex(id))
            .filter(|n| natural.contains(n) && policy == "copy_before_post");
        Some(IterationPhase {
            loop_node: owner.into(),
            symbol: symbol.into(),
            first_declaration: source.to_string(),
            copy_before_post: post.map(|v| f.id(v).to_owned()),
            last_direct_writes: post
                .map(|p| writes_before(f, p, &writes))
                .unwrap_or_default(),
        })
    }
}

fn observations<'a>(function: &'a Function, flow: &Flow<'_>) -> Vec<(Vertex, &'a Node)> {
    function
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Closure | NodeKind::Reference))
        .filter_map(|node| Some((flow.vertex(&node.id)?, node)))
        .collect()
}

fn backedges(flow: &Flow<'_>) -> HashMap<Vertex, Vec<Vertex>> {
    let mut tails: HashMap<Vertex, Vec<Vertex>> = HashMap::new();
    for (tail, head) in flow.back_edges() {
        tails.entry(head).or_default().push(tail);
    }
    tails
}

pub(super) fn derive(
    function: &Function,
    flow: &Flow<'_>,
    dom: &Tree,
) -> (Vec<IterationStorage>, Vec<IterationPhase>) {
    let Some(facts) = bindings(function) else {
        return (Vec::new(), Vec::new());
    };
    let accesses = function
        .accesses
        .iter()
        .map(|a| (a.node.as_str(), a.symbol.as_str()))
        .collect();
    let tails = backedges(flow);
    let sites = observations(function, flow);
    let mut out = Vec::new();
    let mut phases = Vec::new();
    let frame = PhaseFrame {
        function,
        graph: flow,
        dom,
    };
    for (loop_node, fact) in facts {
        let Some(head) = flow.vertex(fact.head) else {
            continue;
        };
        let Some(tails) = tails.get(&head) else {
            continue;
        };
        let natural_loop = members(flow, dom, head, tails);
        for (&symbol, &policy) in &fact.bindings {
            if let Some(phase) = frame.phase((loop_node, symbol, &fact), policy, &natural_loop) {
                phases.push(phase);
            }
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
    (out, phases)
}
