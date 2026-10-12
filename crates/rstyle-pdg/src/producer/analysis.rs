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
    let post = if cyclic {
        divergence_postdominators(&reachable, &successors)
    } else {
        dominators(&reachable, &successors, "X")
    };
    let (definition_in, definition_out) = reaching(graph, &reachable, &predecessors);
    let edges = data_edges(graph, &reachable, &definition_in)?;
    let execution = execution_edges(graph, &reachable, &dom);
    let control = if cyclic {
        cyclic_control_edges(graph, &reachable, &post)
    } else {
        control_edges(graph, &reachable, &post)
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

fn control_edges(graph: &Graph, reached: &BTreeSet<String>, post: &Sets) -> Vec<Dependence> {
    graph
        .transfers
        .iter()
        .filter(|e| {
            reached.contains(&e.source)
                && matches!(e.kind.as_str(), "true" | "false")
                && !post[&e.source].contains(&e.target)
        })
        .flat_map(|transfer| {
            let stop = immediate(&transfer.source, post);
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
                current = immediate(&id, post);
            }
            edges
        })
        .collect()
}

fn divergence_postdominators(vertices: &BTreeSet<String>, successors: &Sets) -> Sets {
    let mut result: Sets = vertices
        .iter()
        .map(|id| (id.clone(), BTreeSet::new()))
        .collect();
    for candidate in vertices {
        let mut must_encounter = BTreeSet::from([candidate.clone()]);
        loop {
            let next: BTreeSet<_> = must_encounter
                .iter()
                .cloned()
                .chain(
                    successors
                        .iter()
                        .filter(|(_, next)| !next.is_empty() && next.is_subset(&must_encounter))
                        .map(|(id, _)| id.clone()),
                )
                .collect();
            if next == must_encounter {
                break;
            }
            must_encounter = next;
        }
        for vertex in must_encounter {
            result
                .get_mut(&vertex)
                .expect("reachable vertex")
                .insert(candidate.clone());
        }
    }
    result
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
