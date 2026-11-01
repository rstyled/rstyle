use super::*;

pub(super) fn neighbours(graph: &Graph, reached: &BTreeSet<String>, forward: bool) -> Sets {
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

pub(super) fn check_acyclic_exit(graph: &Graph, reached: &BTreeSet<String>) -> Result<(), String> {
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

pub(super) fn dominators(vertices: &BTreeSet<String>, neighbours: &Sets, root: &str) -> Sets {
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

pub(super) fn immediate(id: &str, sets: &Sets) -> Option<String> {
    sets.get(id)?
        .iter()
        .filter(|candidate| candidate.as_str() != id)
        .max_by_key(|candidate| sets[*candidate].len())
        .cloned()
}

pub(super) fn reaching(
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

pub(super) fn union_definitions(
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

pub(super) fn transfer_definitions(
    graph: &Graph,
    id: &str,
    incoming: &DefinitionSet,
) -> DefinitionSet {
    let mut result = incoming.clone();
    let node = graph.nodes.iter().find(|n| n.id == id);
    if let Some((symbol, definition)) =
        node.and_then(|n| n.symbol.as_ref().zip(n.definition.as_ref()))
    {
        result.insert(symbol.clone(), BTreeSet::from([definition.clone()]));
    }

    result
}

pub(super) fn data_edges(
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
    let values = value_edges(graph.nodes.iter().filter(|node| reached.contains(&node.id)));
    let alternatives = alternative_edges(graph, |id| reached.contains(id));

    Ok(reads?
        .into_iter()
        .flatten()
        .chain(values)
        .chain(alternatives)
        .collect())
}

pub(super) fn read_edges(
    graph: &Graph,
    node: &Operation,
    incoming: &DefinitionSet,
) -> Result<Vec<Dependence>, String> {
    if matches!(
        node.construct.as_deref(),
        Some(
            "data.indirect-field"
                | "data.indirect-deref"
                | "data.index"
                | "data.option-payload"
                | "iterator.byte-position-prefix"
                | "slice.prefix-byte"
                | "data.mutable-scalar-load"
        )
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

pub(super) fn execution_edges(
    graph: &Graph,
    reached: &BTreeSet<String>,
    dom: &Sets,
) -> Vec<Dependence> {
    let mut dependencies = Vec::new();
    for node in &graph.nodes {
        if !reached.contains(&node.id) {
            continue;
        }
        let Some(source) = immediate(&node.id, dom).filter(|source| source != "E") else {
            continue;
        };
        dependencies.push(edge(
            &source,
            &node.id,
            EdgeFamily::Execution,
            "nearest_dominating_operation",
        ));
    }
    dependencies
}

pub(super) fn cyclic_control_edges(
    graph: &Graph,
    reached: &BTreeSet<String>,
    post: &Sets,
) -> Vec<Dependence> {
    let exceptional: BTreeSet<_> = graph
        .transfers
        .iter()
        .filter(|transfer| transfer.kind == "exception")
        .map(|transfer| transfer.source.as_str())
        .collect();
    graph
        .transfers
        .iter()
        .filter(|transfer| {
            reached.contains(&transfer.source)
                && reached.contains(&transfer.target)
                && (matches!(transfer.kind.as_str(), "true" | "false")
                    || matches!(transfer.kind.as_str(), "normal" | "exception")
                        && exceptional.contains(transfer.source.as_str()))
        })
        .flat_map(|transfer| {
            post[&transfer.target]
                .iter()
                .filter(move |id| {
                    (id.as_str() == transfer.source || !post[&transfer.source].contains(*id))
                        && !matches!(id.as_str(), "E" | "X")
                        && !id.starts_with("P:")
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

pub(super) fn control_edges(
    graph: &Graph,
    reached: &BTreeSet<String>,
    post: &PostTree,
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

pub(super) fn acyclic_postdominators(vertices: &BTreeSet<String>, successors: &Sets) -> PostTree {
    let graph = Topology::new(vertices, successors);
    let order = graph.exit_order();

    PostTree::new(&graph, &order)
}

pub(super) fn divergence_postdominators(vertices: &BTreeSet<String>, successors: &Sets) -> Sets {
    let graph = Topology::new(vertices, successors);

    graph.materialize(&graph.divergence_relation())
}
