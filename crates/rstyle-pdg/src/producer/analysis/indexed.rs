use super::*;
use std::collections::HashMap;

pub(super) fn analyze(graph: &Graph, cyclic: bool) -> Result<Analysis, String> {
    let cyclic = cyclic
        || graph
            .transfers
            .iter()
            .any(|transfer| transfer.kind == "exception");
    let topology = Topology::from_graph(graph);
    let order = if cyclic {
        Vec::new()
    } else {
        exit_order(&topology)?
    };
    let operations = Operations::new(graph, &topology);
    let flow = flow::Flow::new(graph, &topology, &operations.first);
    let reachable: BTreeSet<_> = topology.ids.iter().cloned().collect();
    let mut edges = data_edges(graph, &topology, &operations, &flow)?;
    edges.extend(execution_edges(&operations, &topology));
    edges.extend(if cyclic {
        cyclic_edges(graph, &topology, &topology.divergence_relation())
    } else {
        control_edges(graph, &topology, &PostTree::new(&topology, &order))
    });

    Ok(Analysis {
        edges,
        reachable,
        definition_in: flow.materialize(&topology, &flow.incoming),
        definition_out: flow.materialize(&topology, &flow.outgoing),
    })
}

struct Occurrence<'a> {
    vertex: usize,
    node: &'a Operation,
}

struct Operations<'a> {
    first: Vec<Option<&'a Operation>>,
    occurrences: Vec<Occurrence<'a>>,
}

impl<'a> Operations<'a> {
    fn new(graph: &'a Graph, topology: &Topology) -> Self {
        let mut result = Self {
            first: vec![None; topology.ids.len()],
            occurrences: Vec::new(),
        };
        for node in &graph.nodes {
            if let Some(&vertex) = topology.indices.get(&node.id) {
                result.first[vertex].get_or_insert(node);
                result.occurrences.push(Occurrence { vertex, node });
            }
        }
        result
    }
}

fn exit_order(topology: &Topology) -> Result<Vec<usize>, String> {
    let order = topology.admitted_exit();
    if order.len() == topology.ids.len() {
        Ok(order)
    } else {
        Err(
            "Reachable normal flow must be acyclic and every vertex must reach the normal exit."
                .into(),
        )
    }
}

fn data_edges(
    graph: &Graph,
    topology: &Topology,
    operations: &Operations<'_>,
    flow: &flow::Flow<'_>,
) -> Result<Vec<Dependence>, String> {
    let mut edges = Vec::new();
    for occurrence in &operations.occurrences {
        if occurrence.node.category == NodeCategory::Read {
            edges.extend(read_edges(
                occurrence.node,
                &flow.incoming[occurrence.vertex],
                flow,
            )?);
        }
    }

    edges.extend(value_edges(
        operations
            .occurrences
            .iter()
            .map(|occurrence| occurrence.node),
    ));
    edges.extend(alternative_edges(graph, |id| {
        topology.indices.contains_key(id)
    }));
    Ok(edges)
}

fn indirect_read(node: &Operation) -> bool {
    node.symbol.is_none()
        && matches!(
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
        )
}

fn read_edges(
    node: &Operation,
    incoming: &bits::Bits,
    flow: &flow::Flow<'_>,
) -> Result<Vec<Dependence>, String> {
    if indirect_read(node) {
        return Ok(Vec::new());
    }
    let symbol = node
        .symbol
        .as_deref()
        .ok_or_else(|| "Read without lexical binding".to_owned())?;
    let definitions: Vec<_> = flow
        .symbols
        .get(symbol)
        .into_iter()
        .flatten()
        .copied()
        .filter(|&definition| incoming.contains(definition))
        .collect();
    if definitions.is_empty() {
        return Err(format!("No reachable definition for read {}", node.id));
    }

    definitions
        .into_iter()
        .map(|index| {
            let definition = flow.pairs[index].1;
            let source = flow
                .sources
                .get(definition)
                .copied()
                .flatten()
                .ok_or_else(|| format!("Unresolved definition {definition}"))?;
            Ok(Dependence {
                definition: Some(definition.to_owned()),
                symbol: Some(symbol.to_owned()),
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

fn execution_edges(operations: &Operations<'_>, topology: &Topology) -> Vec<Dependence> {
    let immediate = dominance::immediate(topology);
    operations
        .occurrences
        .iter()
        .filter_map(|occurrence| {
            let source = &topology.ids[immediate[occurrence.vertex]?];
            (source != "E").then(|| {
                edge(
                    source,
                    &occurrence.node.id,
                    EdgeFamily::Execution,
                    "nearest_dominating_operation",
                )
            })
        })
        .collect()
}

fn control_edges(graph: &Graph, topology: &Topology, post: &PostTree) -> Vec<Dependence> {
    let mut edges = Vec::new();
    for transfer in &graph.transfers {
        let Some(&source) = topology.indices.get(&transfer.source) else {
            continue;
        };
        let target = topology.indices[&transfer.target];
        if !matches!(transfer.kind.as_str(), "true" | "false")
            || post.contains_index(source, target)
        {
            continue;
        }
        let stop = post.immediate_index(source);
        let mut current = Some(target);
        while current.is_some() && current != stop {
            let vertex = current.expect("current postdominator");
            if topology.ids[vertex] != "X" {
                edges.push(Dependence {
                    outcome: Some(transfer.kind.clone()),
                    ..edge(
                        &transfer.source,
                        &topology.ids[vertex],
                        EdgeFamily::Control,
                        "postdominance",
                    )
                });
            }
            current = post.immediate_index(vertex);
        }
    }
    edges
}

fn cyclic_edges(graph: &Graph, topology: &Topology, post: &[Vec<usize>]) -> Vec<Dependence> {
    let exceptional: HashMap<_, _> = graph
        .transfers
        .iter()
        .filter(|transfer| transfer.kind == "exception")
        .map(|transfer| (transfer.source.as_str(), ()))
        .collect();
    let mut edges = Vec::new();
    for transfer in &graph.transfers {
        let Some(&source) = topology.indices.get(&transfer.source) else {
            continue;
        };
        if !(matches!(transfer.kind.as_str(), "true" | "false")
            || matches!(transfer.kind.as_str(), "normal" | "exception")
                && exceptional.contains_key(transfer.source.as_str()))
        {
            continue;
        }
        let target = topology.indices[&transfer.target];
        for &candidate in &post[target] {
            let id = &topology.ids[candidate];
            if (candidate == source || post[source].binary_search(&candidate).is_err())
                && !matches!(id.as_str(), "E" | "X")
                && !id.starts_with("P:")
            {
                edges.push(Dependence {
                    outcome: Some(transfer.kind.clone()),
                    ..edge(
                        &transfer.source,
                        id,
                        EdgeFamily::Control,
                        "divergence_sensitive_postdominance",
                    )
                });
            }
        }
    }
    edges
}
