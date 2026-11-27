//! Join synchronous Go call sites to normal continuations. An exit is a
//! conditional return boundary. Listing it does not assert that the callee returns.
use super::{Derived, Flow, each_reached};
use crate::cpg::model::{
    DefinitionKind, Dispatch, Function, Node, NodeKind, RefStatus, TargetKind,
};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallReturn {
    pub definition: String,
    pub continuations: Vec<String>,
    pub local: Option<Boundary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Boundary {
    pub entry: String,
    pub normal_exits: Vec<String>,
}

fn resolved<'a>(tables: &'a Tables<'a>, body: &Body<'_>, node: &Node) -> Option<&'a str> {
    // A normal continuation has Go call semantics for an actual call expression
    // and for the frontend's implicit initializer invocations.
    if !matches!(
        node.original_kind.as_str(),
        "CallExpr" | "PackageInit.call" | "ProgramStart.call"
    ) {
        return None;
    }
    let reference = body.reference(&node.id)?;
    if reference.status != RefStatus::Resolved || reference.dispatch != Some(Dispatch::Static) {
        return None;
    }
    let target = reference.target.as_ref()?;
    let callee = body.reference(node.children.first()?)?;
    let operand = callee.target.as_ref()?;
    if target.kind != TargetKind::Definition
        || callee.status != RefStatus::Resolved
        || operand.kind != target.kind
        || operand.id != target.id
    {
        return None;
    }
    let def = tables.definition(target.id.as_deref()?).ok()?;
    (def.kind == DefinitionKind::Function).then_some(&def.id)
}

fn continuation(graph: &Flow<'_>, site: &str) -> Vec<String> {
    graph
        .vertex(site)
        .into_iter()
        .flat_map(|i| &graph.edges[i])
        .filter(|(_, kind, _)| *kind == "normal")
        .map(|(v, _, _)| graph.id(*v).to_owned())
        .collect()
}

pub(crate) fn attach(tables: &Tables<'_>, function: &Function, derived: &mut Derived) {
    // Static dispatch alone is insufficient for another language's async functions.
    if tables.language() != "go"
        || !function
            .refs
            .iter()
            .any(|r| r.dispatch == Some(Dispatch::Static))
    {
        return;
    }
    let Ok(body) = Body::new(function) else {
        return;
    };
    let postponed: HashSet<_> = function
        .nodes
        .iter()
        .filter(|n| matches!(n.extension.as_deref(), Some("go:go" | "go:defer")))
        .flat_map(|n| &n.children)
        .collect();
    let calls: Vec<_> = function
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Call && !postponed.contains(&n.id))
        .filter_map(|n| Some((&n.id, resolved(tables, &body, n)?)))
        .collect();
    if calls.is_empty() {
        return;
    }
    each_reached(function, derived, |g, reached, own| {
        for (id, def) in &calls {
            if reached.contains(id.as_str()) {
                let local = tables.function(def).map(|f| Boundary {
                    entry: f.cfg.entry.clone(),
                    normal_exits: f
                        .cfg
                        .exits
                        .iter()
                        .filter(|e| e.kind == "normal")
                        .map(|e| e.id.clone())
                        .collect(),
                });
                own.call_returns.insert(
                    (*id).clone(),
                    CallReturn {
                        definition: (*def).to_owned(),
                        continuations: continuation(g, id),
                        local,
                    },
                );
            }
        }
    });
}
