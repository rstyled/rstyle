//! Binding dataflow, dominance, and one-exit postdominance on normal flow.

use super::{Graph, Operation};
use crate::{EdgeFamily, NodeCategory};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type DefinitionSet = BTreeMap<String, BTreeSet<String>>;
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
    let reachable = reachability(graph);
    if !cyclic {
        check_acyclic_exit(graph, &reachable)?;
    }
    let predecessors = neighbours(graph, &reachable, false);
    let successors = neighbours(graph, &reachable, true);
    let dom = dominators(&reachable, &predecessors, "E");
    let (definition_in, definition_out) = reaching(graph, &reachable, &predecessors);
    let edges = data_edges(graph, &reachable, &definition_in)?;
    let execution = execution_edges(graph, &reachable, &dom);
    let control = if cyclic {
        cyclic_control_edges(
            graph,
            &reachable,
            &divergence_postdominators(&reachable, &successors),
        )
    } else {
        control_edges(
            graph,
            &reachable,
            &acyclic_postdominators(&reachable, &successors),
        )
    };

    Ok(Analysis {
        edges: edges.into_iter().chain(execution).chain(control).collect(),
        reachable,
        definition_in,
        definition_out,
    })
}

pub fn reachability(graph: &Graph) -> BTreeSet<String> {
    let mut reached = BTreeSet::from(["E".to_string()]);
    loop {
        let next: BTreeSet<_> = reached
            .iter()
            .cloned()
            .chain(
                graph
                    .transfers
                    .iter()
                    .filter(|e| reached.contains(&e.source))
                    .map(|e| e.target.clone()),
            )
            .collect();
        if next == reached {
            return reached;
        }
        reached = next;
    }
}

fn neighbours(graph: &Graph, reached: &BTreeSet<String>, forward: bool) -> Sets {
    reached
        .iter()
        .map(|id| {
            let edges = graph
                .transfers
                .iter()
                .filter(|e| reached.contains(&e.source) && reached.contains(&e.target));
            let nodes = edges
                .filter(|e| {
                    if forward {
                        &e.source == id
                    } else {
                        &e.target == id
                    }
                })
                .map(|e| {
                    if forward {
                        e.target.clone()
                    } else {
                        e.source.clone()
                    }
                })
                .collect();
            (id.clone(), nodes)
        })
        .collect()
}

fn check_acyclic_exit(graph: &Graph, reached: &BTreeSet<String>) -> Result<(), String> {
    let successors = neighbours(graph, reached, true);
    let mut removed = BTreeSet::from(["X".to_string()]);
    loop {
        let next = removed
            .iter()
            .cloned()
            .chain(
                successors
                    .iter()
                    .filter(|(_, successors)| {
                        !successors.is_empty() && successors.is_subset(&removed)
                    })
                    .map(|(id, _)| id.clone()),
            )
            .collect();
        if next == removed {
            break;
        }
        removed = next;
    }

    if reached.is_subset(&removed) {
        Ok(())
    } else {
        Err(
            "Reachable normal flow must be acyclic and every vertex must reach the normal exit."
                .to_string(),
        )
    }
}

fn dominators(vertices: &BTreeSet<String>, neighbours: &Sets, root: &str) -> Sets {
    let mut sets: Sets = vertices
        .iter()
        .map(|id| {
            (
                id.clone(),
                if id == root {
                    BTreeSet::from([id.clone()])
                } else {
                    vertices.clone()
                },
            )
        })
        .collect();
    loop {
        let next: Sets = vertices
            .iter()
            .map(|id| {
                let incoming = &neighbours[id];
                let set = if id == root {
                    BTreeSet::from([id.clone()])
                } else {
                    let shared = incoming.iter().fold(vertices.clone(), |shared, p| {
                        shared.intersection(&sets[p]).cloned().collect()
                    });
                    shared
                        .into_iter()
                        .chain(std::iter::once(id.clone()))
                        .collect()
                };
                (id.clone(), set)
            })
            .collect();
        if next == sets {
            return sets;
        }
        sets = next;
    }
}

fn immediate(id: &str, sets: &Sets) -> Option<String> {
    sets.get(id)?
        .iter()
        .filter(|candidate| candidate.as_str() != id)
        .max_by_key(|candidate| sets[*candidate].len())
        .cloned()
}

fn reaching(
    graph: &Graph,
    reached: &BTreeSet<String>,
    predecessors: &Sets,
) -> (
    BTreeMap<String, DefinitionSet>,
    BTreeMap<String, DefinitionSet>,
) {
    let inputs: DefinitionSet = graph
        .nodes
        .iter()
        .filter(|n| n.category == NodeCategory::FormalInput)
        .filter_map(|n| Some((n.symbol.clone()?, BTreeSet::from([n.definition.clone()?]))))
        .collect();
    let mut outputs: BTreeMap<String, DefinitionSet> = reached
        .iter()
        .map(|id| (id.clone(), DefinitionSet::new()))
        .collect();
    outputs.insert("E".to_string(), inputs.clone());
    loop {
        let incoming: BTreeMap<_, _> = reached
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    if id == "E" {
                        inputs.clone()
                    } else {
                        union_definitions(&predecessors[id], &outputs)
                    },
                )
            })
            .collect();
        let next: BTreeMap<_, _> = incoming
            .iter()
            .map(|(id, set)| (id.clone(), transfer_definitions(graph, id, set)))
            .collect();
        if next == outputs {
            return (incoming, outputs);
        }
        outputs = next;
    }
}

fn union_definitions(
    predecessors: &BTreeSet<String>,
    outputs: &BTreeMap<String, DefinitionSet>,
) -> DefinitionSet {
    let mut result = DefinitionSet::new();
    for predecessor in predecessors {
        for (symbol, definitions) in &outputs[predecessor] {
            result
                .entry(symbol.clone())
                .or_default()
                .extend(definitions.iter().cloned());
        }
    }

    result
}

fn transfer_definitions(graph: &Graph, id: &str, incoming: &DefinitionSet) -> DefinitionSet {
    let mut result = incoming.clone();
    let node = graph.nodes.iter().find(|n| n.id == id);
    if let Some((symbol, definition)) =
        node.and_then(|n| n.symbol.as_ref().zip(n.definition.as_ref()))
    {
        result.insert(symbol.clone(), BTreeSet::from([definition.clone()]));
    }

    result
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

fn data_edges(
    graph: &Graph,
    reached: &BTreeSet<String>,
    incoming: &BTreeMap<String, DefinitionSet>,
) -> Result<Vec<Dependence>, String> {
    let reads: Result<Vec<Vec<_>>, _> = graph
        .nodes
        .iter()
        .filter(|n| reached.contains(&n.id) && n.category == NodeCategory::Read)
        .map(|n| read_edges(graph, n, &incoming[&n.id]))
        .collect();
    let values = value_edges(graph, reached);
    let alternatives = alternative_edges(graph, reached);

    Ok(reads?
        .into_iter()
        .flatten()
        .chain(values)
        .chain(alternatives)
        .collect())
}

fn value_edges(graph: &Graph, reached: &BTreeSet<String>) -> Vec<Dependence> {
    let values = graph
        .nodes
        .iter()
        .filter(|n| reached.contains(&n.id))
        .flat_map(|n| {
            n.operands
                .iter()
                .enumerate()
                .map(move |(position, value)| Dependence {
                    position: Some(position as u64),
                    ..edge(value, &n.id, EdgeFamily::Data, "value_operand")
                })
        });

    values.collect()
}

fn alternative_edges(graph: &Graph, reached: &BTreeSet<String>) -> Vec<Dependence> {
    let alternatives = graph
        .value_sites
        .iter()
        .filter(|s| {
            s.cfg_successor_node_id
                .as_ref()
                .is_some_and(|id| reached.contains(id))
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

fn read_edges(
    graph: &Graph,
    node: &Operation,
    incoming: &DefinitionSet,
) -> Result<Vec<Dependence>, String> {
    if matches!(
        node.construct.as_deref(),
        Some("data.indirect-field" | "data.indirect-deref")
    ) && node.symbol.is_none()
    {
        return Ok(Vec::new());
    }
    let symbol = node
        .symbol
        .as_ref()
        .ok_or_else(|| "Read without lexical binding".to_string())?;
    let definitions = incoming
        .get(symbol)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("No reachable definition for read {}", node.id))?;

    definitions
        .iter()
        .map(|definition| {
            let source = graph
                .nodes
                .iter()
                .find(|n| n.definition.as_ref() == Some(definition))
                .ok_or_else(|| format!("Unresolved definition {definition}"))?;
            Ok(Dependence {
                definition: Some(definition.clone()),
                symbol: Some(symbol.clone()),
                position: Some(0),
                ..edge(
                    &source.id,
                    &node.id,
                    EdgeFamily::Data,
                    "reaching_definition",
                )
            })
        })
        .collect()
}

fn execution_edges(graph: &Graph, reached: &BTreeSet<String>, dom: &Sets) -> Vec<Dependence> {
    graph
        .nodes
        .iter()
        .filter(|n| reached.contains(&n.id))
        .filter_map(|n| {
            let source = immediate(&n.id, dom)?;
            (source != "E").then(|| {
                edge(
                    &source,
                    &n.id,
                    EdgeFamily::Execution,
                    "nearest_dominating_operation",
                )
            })
        })
        .collect()
}

fn control_edges(
    graph: &Graph,
    reached: &BTreeSet<String>,
    post: &PostTree<'_>,
) -> Vec<Dependence> {
    graph
        .transfers
        .iter()
        .filter(|e| {
            reached.contains(&e.source)
                && matches!(e.kind.as_str(), "true" | "false")
                && !post.contains(&e.source, &e.target)
        })
        .flat_map(|transfer| {
            let stop = post.immediate(&transfer.source);
            let mut current = Some(transfer.target.clone());
            let mut edges = Vec::new();
            while current.is_some() && current != stop {
                let id = current.expect("current postdominator");
                if id != "X" {
                    edges.push(Dependence {
                        outcome: Some(transfer.kind.clone()),
                        ..edge(&transfer.source, &id, EdgeFamily::Control, "postdominance")
                    });
                }
                current = post.immediate(&id);
            }
            edges
        })
        .collect()
}

// Indexed once per analysis; Sets already deduplicate outcome-labelled transfers.
struct PostGraph<'a> {
    ids: Vec<&'a String>,
    successors: Vec<Vec<usize>>,
    predecessors: Vec<Vec<usize>>,
}

impl<'a> PostGraph<'a> {
    fn new(vertices: &'a BTreeSet<String>, successors: &Sets) -> Self {
        let ids: Vec<_> = vertices.iter().collect();
        let indices: BTreeMap<_, _> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        let next: Vec<Vec<_>> = ids
            .iter()
            .map(|id| {
                successors[*id]
                    .iter()
                    .map(|target| indices[target])
                    .collect()
            })
            .collect();
        let mut predecessors = vec![Vec::new(); ids.len()];
        for (source, targets) in next.iter().enumerate() {
            for &target in targets {
                predecessors[target].push(source);
            }
        }

        Self {
            ids,
            successors: next,
            predecessors,
        }
    }

    fn materialize(&self, relation: &[Vec<usize>]) -> Sets {
        self.ids
            .iter()
            .zip(relation)
            .map(|(id, post)| {
                (
                    (*id).clone(),
                    post.iter().map(|&z| self.ids[z].clone()).collect(),
                )
            })
            .collect()
    }

    fn divergence_relation(&self) -> Vec<Vec<usize>> {
        let degrees: Vec<_> = self.successors.iter().map(Vec::len).collect();
        let mut scratch = Admission::new(degrees.len());
        let mut relation = vec![Vec::new(); degrees.len()];
        for candidate in 0..degrees.len() {
            scratch.seed(candidate, &degrees);
            scratch.propagate(&self.predecessors);
            for &vertex in &scratch.queue {
                relation[vertex].push(candidate);
            }
        }

        relation
    }
}

impl PostGraph<'_> {
    fn exit_order(&self) -> Vec<usize> {
        let exit = self
            .ids
            .iter()
            .position(|id| id.as_str() == "X")
            .expect("validated normal exit");
        let degrees: Vec<_> = self.successors.iter().map(Vec::len).collect();
        let mut admission = Admission::new(self.ids.len());
        admission.seed(exit, &degrees);
        admission.propagate(&self.predecessors);
        assert_eq!(
            admission.queue.len(),
            self.ids.len(),
            "validated acyclic normal flow"
        );

        admission.queue
    }
}

// The postdominator tree stores O(V log V) indices instead of O(R) strings.
struct PostTree<'a> {
    ids: Vec<&'a String>,
    indices: BTreeMap<&'a str, usize>,
    parent: Vec<usize>,
    depth: Vec<usize>,
    jumps: Vec<Vec<usize>>,
}

impl<'a> PostTree<'a> {
    fn new(graph: &PostGraph<'a>, order: &[usize]) -> Self {
        let mut tree = Self::empty(graph, order[0]);
        tree.attach_order(graph, order);

        tree
    }

    fn empty(graph: &PostGraph<'a>, root: usize) -> Self {
        let size = graph.ids.len();
        let levels = (usize::BITS - size.leading_zeros()) as usize;

        Self {
            ids: graph.ids.clone(),
            indices: graph
                .ids
                .iter()
                .enumerate()
                .map(|(i, id)| (id.as_str(), i))
                .collect(),
            parent: vec![root; size],
            depth: vec![0; size],
            jumps: vec![vec![root; levels]; size],
        }
    }

    fn attach_order(&mut self, graph: &PostGraph<'_>, order: &[usize]) {
        for &vertex in &order[1..] {
            let (first, rest) = graph.successors[vertex]
                .split_first()
                .expect("validated successor");
            let parent = rest
                .iter()
                .fold(*first, |shared, &next| self.common_ancestor(shared, next));
            self.attach(vertex, parent);
        }
    }

    fn attach(&mut self, vertex: usize, parent: usize) {
        self.parent[vertex] = parent;
        self.depth[vertex] = self.depth[parent] + 1;
        self.fill_jumps(vertex, parent);
    }

    fn fill_jumps(&mut self, vertex: usize, parent: usize) {
        self.jumps[vertex][0] = parent;
        for level in 1..self.jumps[vertex].len() {
            self.jumps[vertex][level] = self.jumps[self.jumps[vertex][level - 1]][level - 1];
        }
    }

    fn lift(&self, mut vertex: usize, mut distance: usize) -> usize {
        while distance != 0 {
            let level = distance.trailing_zeros() as usize;
            vertex = self.jumps[vertex][level];
            distance &= distance - 1;
        }

        vertex
    }

    fn common_ancestor(&self, left: usize, right: usize) -> usize {
        let (mut left, mut right) = if self.depth[left] >= self.depth[right] {
            (self.lift(left, self.depth[left] - self.depth[right]), right)
        } else {
            (left, self.lift(right, self.depth[right] - self.depth[left]))
        };
        if left == right {
            return left;
        }
        for level in (0..self.jumps[left].len()).rev() {
            if self.jumps[left][level] != self.jumps[right][level] {
                left = self.jumps[left][level];
                right = self.jumps[right][level];
            }
        }

        self.parent[left]
    }

    fn contains(&self, vertex: &str, candidate: &str) -> bool {
        let vertex = self.indices[vertex];
        let candidate = self.indices[candidate];
        self.depth[vertex] >= self.depth[candidate]
            && self.lift(vertex, self.depth[vertex] - self.depth[candidate]) == candidate
    }

    fn immediate(&self, vertex: &str) -> Option<String> {
        let vertex = self.indices[vertex];
        (self.depth[vertex] != 0).then(|| self.ids[self.parent[vertex]].clone())
    }
}

/// Successors precede their non-X source in exit_order. Their postdominator
/// trees are final, so the intersection of their ancestor chains is precisely
/// the ancestor chain of their lowest common ancestor, the source's ipost.
fn acyclic_postdominators<'a>(vertices: &'a BTreeSet<String>, successors: &Sets) -> PostTree<'a> {
    let graph = PostGraph::new(vertices, successors);
    let order = graph.exit_order();

    PostTree::new(&graph, &order)
}

struct Admission {
    remaining: Vec<usize>,
    admitted: Vec<bool>,
    queue: Vec<usize>,
}

impl Admission {
    fn new(size: usize) -> Self {
        Self {
            remaining: vec![0; size],
            admitted: vec![false; size],
            queue: Vec::with_capacity(size),
        }
    }

    fn seed(&mut self, candidate: usize, degrees: &[usize]) {
        self.reset(degrees);
        self.admitted[candidate] = true;
        self.queue.push(candidate);
    }

    fn reset(&mut self, degrees: &[usize]) {
        self.remaining.copy_from_slice(degrees);
        self.admitted.fill(false);
        self.queue.clear();
    }

    fn propagate(&mut self, predecessors: &[Vec<usize>]) {
        let mut head = 0;
        while head < self.queue.len() {
            let vertex = self.queue[head];
            head += 1;
            self.process(&predecessors[vertex]);
        }
    }

    fn process(&mut self, predecessors: &[usize]) {
        for &vertex in predecessors {
            if self.admitted[vertex] {
                continue;
            }
            self.remaining[vertex] -= 1;
            if self.remaining[vertex] == 0 {
                self.admitted[vertex] = true;
                self.queue.push(vertex);
            }
        }
    }
}

/// lfp S . {z} union {v | succ(v) != empty and succ(v) subset S}.
/// For every unadmitted v, remaining[v] counts successors whose queue events
/// have not been processed (not successors that have not yet been admitted).
/// Each admitted vertex has exactly one event. Only z bypasses the counter.
fn divergence_postdominators(vertices: &BTreeSet<String>, successors: &Sets) -> Sets {
    let graph = PostGraph::new(vertices, successors);

    graph.materialize(&graph.divergence_relation())
}

fn cyclic_control_edges(graph: &Graph, reached: &BTreeSet<String>, post: &Sets) -> Vec<Dependence> {
    graph
        .transfers
        .iter()
        .filter(|transfer| {
            reached.contains(&transfer.source)
                && reached.contains(&transfer.target)
                && matches!(transfer.kind.as_str(), "true" | "false")
        })
        .flat_map(|transfer| {
            post[&transfer.target]
                .iter()
                .filter(move |id| {
                    (id.as_str() == transfer.source || !post[&transfer.source].contains(*id))
                        && !matches!(id.as_str(), "E" | "X")
                })
                .map(move |id| Dependence {
                    outcome: Some(transfer.kind.clone()),
                    ..edge(
                        &transfer.source,
                        id,
                        EdgeFamily::Control,
                        "divergence_sensitive_postdominance",
                    )
                })
        })
        .collect()
}

#[cfg(test)]
#[path = "analysis_tests.rs"]
mod tests;
