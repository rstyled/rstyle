//! Effects on future visits from mutations of the range's original map.
//! Eligibility is conditional on whether the key was absent or still unvisited.
use super::Flow;
use crate::cpg::model::{AccessMode, Function, Node, NodeKind};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Update {
    pub key: String,
    pub binding: String,
    pub condition: String,
    pub later_visit: Vec<String>,
}

impl Update {
    fn at(binding: &str, key: &str, store: bool) -> Self {
        let (condition, later_visit) = if store {
            ("new_entry", vec!["skip".into(), "visit".into()])
        } else {
            ("unvisited_entry_while_deleted", vec!["skip".into()])
        };
        Self {
            binding: binding.into(),
            key: key.into(),
            condition: condition.into(),
            later_visit,
        }
    }
}

fn mutation<'a>(
    body: &Body<'a>,
    tables: &Tables<'_>,
    node: &Node,
) -> Option<(&'a Node, &'a Node, bool)> {
    match node.kind {
        NodeKind::Assignment => {
            let idx = body.child(node, 0).ok()?;
            if idx.kind != NodeKind::IndexAccess {
                return None;
            }
            Some((body.child(idx, 0).ok()?, body.child(idx, 1).ok()?, true))
        }
        NodeKind::Call => {
            let r = body.reference(&node.id)?;
            let def = tables.named_definition(r.target.as_ref()?.id.as_deref())?;
            if !def.standard || def.operation.as_deref() != Some("go:delete") {
                return None;
            }
            Some((body.child(node, 1).ok()?, body.child(node, 2).ok()?, false))
        }
        _ => None,
    }
}

fn repeatable(graph: &Flow<'_>, header: usize) -> HashSet<usize> {
    let tails: Vec<_> = graph.forward.predecessors[header]
        .iter()
        .copied()
        .filter(|v| {
            graph.edges[*v]
                .iter()
                .any(|(to, _, role)| *to == header && *role == Some("back_edge"))
        })
        .collect();
    super::iteration_storage::members(graph, &graph.dominators(), header, &tails)
}

pub(super) fn derive(
    f: &Function,
    graph: &Flow<'_>,
    context: (&Body<'_>, &Tables<'_>),
    head: &Node,
) -> BTreeMap<String, Update> {
    let mut out = BTreeMap::new();
    let Some(binding) = f
        .extensions
        .as_ref()
        .and_then(|x| x.get("go:stable-map-ranges"))
        .and_then(|x| x.get(&head.id))
        .and_then(|x| x.as_str())
    else {
        return out;
    };
    let (body, tables) = context;
    let Some(header) = graph.vertex(&head.id) else {
        return out;
    };
    let Some(source) = head.children.get(1) else {
        return out;
    };
    let reads: HashSet<_> = f
        .accesses
        .iter()
        .filter(|a| a.symbol == binding && a.mode != AccessMode::Write)
        .map(|a| a.node.as_str())
        .collect();
    if !reads.contains(source.as_str()) {
        return out;
    }
    for v in repeatable(graph, header) {
        let id = graph.id(v);
        let Some((base, key, store)) = body.node(id).ok().and_then(|n| mutation(body, tables, n))
        else {
            continue;
        };
        if !reads.contains(base.id.as_str()) {
            continue;
        }
        out.insert(id.into(), Update::at(binding, &key.id, store));
    }
    out
}
