//! Serialized cyclic-flow proof using finite fixed points, never bounded paths.

use crate::*;
use std::collections::{BTreeMap, BTreeSet};

type Definitions = BTreeMap<String, BTreeSet<String>>;
type Flow = BTreeMap<String, Definitions>;
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
    let incoming = definitions(pdg, cfg, &mapping, &reached, &predecessors);
    check_reads(pdg, &mapping, &reached, &incoming)?;
    check_values(pdg, &mapping, &reached, &dominance)?;

    Ok(BTreeSet::new())
}

fn predecessors(cfg: &CfgDocument, reached: &BTreeSet<String>) -> Sets {
    reached
        .iter()
        .map(|id| {
            (
                id.clone(),
                cfg.edges
                    .iter()
                    .filter(|edge| &edge.target == id && reached.contains(&edge.source))
                    .map(|edge| edge.source.clone())
                    .collect(),
            )
        })
        .collect()
}

fn dominance(cfg: &CfgDocument, reached: &BTreeSet<String>, predecessors: &Sets) -> Sets {
    let mut sets: Sets = reached
        .iter()
        .map(|id| {
            (
                id.clone(),
                if id == &cfg.entry_node_id {
                    BTreeSet::from([id.clone()])
                } else {
                    reached.clone()
                },
            )
        })
        .collect();
    loop {
        let next: Sets = reached
            .iter()
            .map(|id| {
                let mut common = if id == &cfg.entry_node_id {
                    BTreeSet::new()
                } else {
                    reached.clone()
                };
                for predecessor in &predecessors[id] {
                    common.retain(|candidate| sets[predecessor].contains(candidate));
                }
                common.insert(id.clone());
                (id.clone(), common)
            })
            .collect();
        if sets == next {
            return sets;
        }
        sets = next;
    }
}

fn definitions(
    pdg: &PdgDocument,
    cfg: &CfgDocument,
    mapping: &[OperationMapEntry],
    reached: &BTreeSet<String>,
    predecessors: &Sets,
) -> Flow {
    let inputs: Definitions = pdg
        .nodes
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
        .collect();
    let mut outputs: Flow = reached
        .iter()
        .map(|id| (id.clone(), Definitions::new()))
        .collect();
    loop {
        let incoming: Flow = reached
            .iter()
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
            .collect();
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
        for access in &node.attributes.reads {
            let symbol = access
                .symbol_id
                .as_ref()
                .ok_or_else(|| vec!["Cyclic read lacks a binding".into()])?;
            let definitions = incoming[&entry.cfg_node_id]
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

fn check_values(
    pdg: &PdgDocument,
    mapping: &[OperationMapEntry],
    reached: &BTreeSet<String>,
    dominance: &Sets,
) -> Result<(), Vec<String>> {
    for edge in pdg
        .edges
        .iter()
        .filter(|edge| edge.subkind == "value_operand")
    {
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
        {
            return Err(vec!["Unit cyclic value operand".into()]);
        }
        if node.category == NodeCategory::FormalInput {
            continue;
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
    }
    Ok(())
}
