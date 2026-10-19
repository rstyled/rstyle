//! Serialized cyclic-flow proof using finite fixed points, never bounded paths.

use crate::*;
use std::collections::{BTreeMap, BTreeSet};

type Definitions = BTreeMap<String, BTreeSet<String>>;
type Flow = BTreeMap<String, Definitions>;
type ReadEdge = (String, String, Option<String>, Option<String>, Option<u64>);
type Sets = BTreeMap<String, BTreeSet<String>>;

pub fn validate(pdg: &PdgDocument, cfg: &CfgDocument) -> Result<BTreeSet<String>, Vec<String>> {
    if pdg
        .nodes
        .iter()
        .any(|node| node.category == NodeCategory::Merge)
        || pdg
            .extensions
            .get("compat.rust.hir-r0:value-sites")
            .is_none_or(|sites| sites.as_array().is_none_or(|sites| !sites.is_empty()))
    {
        return Err(vec![
            "Cyclic policy excludes expression value merges".into(),
        ]);
    }
    let mapping = read_operation_map(&pdg.extensions).map_err(|e| vec![e])?;
    let reached = reachable_vertices(cfg);
    let predecessors = predecessors(cfg, &reached);
    let dominance = dominance(cfg, &reached, &predecessors);
    let incoming = definitions(pdg, cfg, &mapping, &predecessors);
    check_reads(pdg, &mapping, &reached, &incoming)?;
    check_values(pdg, &mapping, &reached, &dominance)?;

    Ok(BTreeSet::new())
}

fn predecessors(cfg: &CfgDocument, reached: &BTreeSet<String>) -> Sets {
    let mut sets: Sets = reached
        .iter()
        .map(|id| (id.clone(), BTreeSet::new()))
        .collect();
    for edge in &cfg.edges {
        if reached.contains(&edge.source)
            && let Some(incoming) = sets.get_mut(&edge.target)
        {
            incoming.insert(edge.source.clone());
        }
    }

    sets
}

// Independent worklist proof: unlike producer iteration, update only successors
// of a changed dominance set. Every change strictly removes finite candidates.
fn dominance(cfg: &CfgDocument, reached: &BTreeSet<String>, predecessors: &Sets) -> Sets {
    let mut sets: Sets = reached
        .iter()
        .map(|id| (id.clone(), reached.clone()))
        .collect();
    sets.insert(
        cfg.entry_node_id.clone(),
        BTreeSet::from([cfg.entry_node_id.clone()]),
    );
    let mut pending = reached.clone();
    pending.remove(&cfg.entry_node_id);
    while let Some(id) = pending.pop_first() {
        let next = dominance_step(&id, reached, predecessors, &sets);
        if next != sets[&id] {
            sets.insert(id.clone(), next);
            pending.extend(
                cfg.edges
                    .iter()
                    .filter(|edge| {
                        edge.source == id
                            && edge.target != cfg.entry_node_id
                            && reached.contains(&edge.target)
                    })
                    .map(|edge| edge.target.clone()),
            );
        }
    }

    sets
}

fn dominance_step(
    id: &str,
    reached: &BTreeSet<String>,
    predecessors: &Sets,
    sets: &Sets,
) -> BTreeSet<String> {
    let mut next = reached.clone();
    for predecessor in &predecessors[id] {
        next.retain(|candidate| sets[predecessor].contains(candidate));
    }
    next.insert(id.to_string());

    next
}

fn definitions(
    pdg: &PdgDocument,
    cfg: &CfgDocument,
    mapping: &[OperationMapEntry],
    predecessors: &Sets,
) -> Flow {
    let reached: BTreeSet<_> = predecessors.keys().cloned().collect();
    let inputs = formal_definitions(pdg);
    let mut outputs: Flow = reached
        .iter()
        .map(|id| (id.clone(), Definitions::new()))
        .collect();
    loop {
        let incoming = incoming_definitions(cfg, &inputs, predecessors, &outputs);
        let mut next = incoming.clone();
        for entry in mapping
            .iter()
            .filter(|entry| reached.contains(&entry.cfg_node_id))
        {
            for definition in pdg
                .definitions
                .iter()
                .filter(|definition| definition.node_id == entry.pdg_node_id)
            {
                if let Some(symbol) = &definition.symbol_id {
                    next.get_mut(&entry.cfg_node_id)
                        .expect("reachable mapped operation")
                        .insert(symbol.clone(), BTreeSet::from([definition.id.clone()]));
                }
            }
        }
        if next == outputs {
            return incoming;
        }
        outputs = next;
    }
}

fn incoming_definitions(
    cfg: &CfgDocument,
    inputs: &Definitions,
    predecessors: &Sets,
    outputs: &Flow,
) -> Flow {
    predecessors
        .keys()
        .map(|id| {
            let mut definitions = Definitions::new();
            if id == &cfg.entry_node_id {
                definitions = inputs.clone();
            } else {
                for predecessor in &predecessors[id] {
                    for (symbol, versions) in &outputs[predecessor] {
                        definitions
                            .entry(symbol.clone())
                            .or_default()
                            .extend(versions.iter().cloned());
                    }
                }
            }
            (id.clone(), definitions)
        })
        .collect()
}

fn formal_definitions(pdg: &PdgDocument) -> Definitions {
    pdg.nodes
        .iter()
        .filter(|node| node.category == NodeCategory::FormalInput)
        .flat_map(|node| {
            pdg.definitions
                .iter()
                .filter(move |definition| definition.node_id == node.id)
        })
        .filter_map(|definition| {
            Some((
                definition.symbol_id.clone()?,
                BTreeSet::from([definition.id.clone()]),
            ))
        })
        .collect()
}

fn check_reads(
    pdg: &PdgDocument,
    mapping: &[OperationMapEntry],
    reached: &BTreeSet<String>,
    incoming: &Flow,
) -> Result<(), Vec<String>> {
    let mut expected = Vec::new();
    for node in pdg
        .nodes
        .iter()
        .filter(|node| node.category == NodeCategory::Read)
    {
        let entry = mapping
            .iter()
            .find(|entry| entry.pdg_node_id == node.id)
            .ok_or_else(|| vec!["Missing cyclic read correspondence".into()])?;
        if !reached.contains(&entry.cfg_node_id) {
            continue;
        }
        expected.extend(expected_reads(pdg, node, &incoming[&entry.cfg_node_id])?);
    }

    let mut actual: Vec<_> = pdg
        .edges
        .iter()
        .filter(|edge| edge.subkind == "reaching_definition")
        .map(|edge| {
            (
                edge.source.clone(),
                edge.target.clone(),
                edge.definition_id.clone(),
                edge.symbol_id.clone(),
                edge.operand_position,
            )
        })
        .collect();
    actual.sort();
    expected.sort();
    if actual == expected {
        Ok(())
    } else {
        Err(vec![
            "Cyclic reaching-definition edges disagree with serialized CFG fixed point".into(),
        ])
    }
}

fn expected_reads(
    pdg: &PdgDocument,
    node: &PdgNode,
    incoming: &Definitions,
) -> Result<Vec<ReadEdge>, Vec<String>> {
    let mut expected = Vec::new();
    for access in &node.attributes.reads {
        let symbol = access
            .symbol_id
            .as_ref()
            .ok_or_else(|| vec!["Cyclic read lacks a binding".into()])?;
        let definitions = incoming
            .get(symbol)
            .filter(|definitions| !definitions.is_empty())
            .ok_or_else(|| vec!["Cyclic read has no reaching definition".into()])?;
        for id in definitions {
            let definition = pdg
                .definitions
                .iter()
                .find(|definition| &definition.id == id)
                .ok_or_else(|| vec!["Cyclic definition does not resolve".into()])?;
            expected.push((
                definition.node_id.clone(),
                node.id.clone(),
                Some(id.clone()),
                Some(symbol.clone()),
                Some(0),
            ));
        }
    }

    Ok(expected)
}

fn check_values(
    pdg: &PdgDocument,
    mapping: &[OperationMapEntry],
    reached: &BTreeSet<String>,
    dominance: &Sets,
) -> Result<(), Vec<String>> {
    let proof = ValueProof {
        pdg,
        mapping,
        reached,
        dominance,
    };
    for edge in pdg
        .edges
        .iter()
        .filter(|edge| edge.subkind == "value_operand")
    {
        check_value(edge, &proof)?;
    }
    Ok(())
}

struct ValueProof<'a> {
    pdg: &'a PdgDocument,
    mapping: &'a [OperationMapEntry],
    reached: &'a BTreeSet<String>,
    dominance: &'a Sets,
}

fn check_value(edge: &PdgEdge, proof: &ValueProof<'_>) -> Result<(), Vec<String>> {
    let ValueProof {
        pdg,
        mapping,
        reached,
        dominance,
    } = proof;

    let consumer = mapping
        .iter()
        .find(|entry| entry.pdg_node_id == edge.target)
        .ok_or_else(|| vec!["Missing cyclic value consumer".into()])?;
    let node = pdg
        .nodes
        .iter()
        .find(|node| node.id == edge.source)
        .ok_or_else(|| vec!["Missing cyclic value producer".into()])?;
    if !reached.contains(&consumer.cfg_node_id) {
        return Err(vec![
            "Unreachable cyclic consumer has executable value dependencies".into(),
        ]);
    }
    if node
        .annotations
        .get("compat.rust.hir-r0:type")
        .and_then(|value| value.get("value"))
        .and_then(serde_json::Value::as_str)
        == Some("unit")
        && !crate::data_aggregates::unit_slot(pdg, edge)
    {
        return Err(vec!["Unit cyclic value operand".into()]);
    }
    if node.category == NodeCategory::FormalInput {
        return Ok(());
    }
    let producer = mapping
        .iter()
        .find(|entry| entry.pdg_node_id == edge.source)
        .ok_or_else(|| vec!["Missing cyclic operand correspondence".into()])?;
    if producer.cfg_node_id == consumer.cfg_node_id
        || !dominance[&consumer.cfg_node_id].contains(&producer.cfg_node_id)
    {
        return Err(vec![
            "Cyclic operand snapshot does not strictly dominate its consumer".into(),
        ]);
    }
    Ok(())
}
