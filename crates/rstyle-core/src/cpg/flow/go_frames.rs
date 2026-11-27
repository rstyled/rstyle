//! Track what crosses a Go function boundary when an invocation is registered
//! for later execution or its results are returned to a caller.

use super::{Derived, Flow};
use crate::cpg::model::{Exit, Function, Node, NodeKind, Type, TypeKind};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Invocation {
    pub call: String,
    pub mode: String,
    pub saved_operands: Vec<String>,
    pub caller_continuations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discarded_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Target {
    pub entry: String,
    pub normal_exits: Vec<String>,
    pub shared_bindings: Vec<String>,
}

fn target(graph: Flow<'_>, exits: &[Exit], shared_bindings: Vec<String>) -> Target {
    let reached = graph.reached();
    Target {
        entry: graph.id(graph.entry()).to_owned(),
        normal_exits: exits
            .iter()
            .filter(|e| e.kind == "normal" && reached.contains(e.id.as_str()))
            .map(|e| e.id.clone())
            .collect(),
        shared_bindings,
    }
}

struct Frame<'a> {
    function: &'a Function,
    body: Body<'a>,
    tables: &'a Tables<'a>,
}

impl Frame<'_> {
    fn target(&self, call: &Node) -> Option<Target> {
        let reference = self.body.reference(&call.id)?;
        if let Some(id) = &reference.closure {
            let arm = self.function.cfg.closures.iter().find(|c| c.node == *id)?;
            let shared = self
                .body
                .node(id)
                .ok()?
                .captures
                .iter()
                .flatten()
                .map(|c| c.symbol.clone())
                .collect();
            return Some(target(Flow::of_closure(arm)?, &arm.exits, shared));
        }
        if reference.dispatch != Some(crate::cpg::model::Dispatch::Static) {
            return None;
        }
        let callee = self
            .tables
            .function(reference.target.as_ref()?.id.as_deref()?)?;
        Some(target(Flow::of(callee)?, &callee.cfg.exits, Vec::new()))
    }

    fn invocation(
        &self,
        node: &Node,
        graph: &Flow<'_>,
        reached: &HashSet<&str>,
    ) -> Option<Invocation> {
        let mode = match node.extension.as_deref()? {
            "go:go" => "goroutine",
            "go:defer" => "defer",
            _ => return None,
        };
        let call = self.body.node(node.children.first()?).ok()?;
        if call.kind != NodeKind::Call
            || call.children.iter().any(|c| !reached.contains(c.as_str()))
        {
            return None;
        }
        // Older captures put the async invocation in the caller's graph.
        if mode == "goroutine" && reached.contains(call.id.as_str()) {
            return None;
        }
        let site = graph.vertex(&node.id)?;
        let ty = self.tables.ty(call.ty.as_deref()?).ok()?;
        Some(Invocation {
            call: call.id.clone(),
            mode: mode.into(),
            saved_operands: call.children.clone(),
            caller_continuations: graph.forward.successors[site]
                .iter()
                .map(|v| graph.id(*v).to_owned())
                .collect(),
            discarded_type: (ty.kind != TypeKind::Unit).then(|| ty.id.clone()),
            target: self.target(call),
        })
    }

    fn result_types(&self, owner: Option<&str>) -> Option<Vec<&Type>> {
        let result = match owner {
            None => self.function.signature.result.as_str(),
            Some(closure) => self
                .tables
                .ty(self.body.node(closure).ok()?.ty.as_deref()?)
                .ok()?
                .result
                .as_deref()?,
        };
        let ty = self.tables.ty(result).ok()?;
        match ty.kind {
            TypeKind::Tuple => ty
                .items
                .as_ref()?
                .iter()
                .map(|id| self.tables.ty(id).ok())
                .collect(),
            TypeKind::Unit => Some(Vec::new()),
            _ => Some(vec![ty]),
        }
    }

    fn slots(&self, owner: Option<&str>) -> Option<Vec<ResultSlot>> {
        let names = self
            .function
            .extensions
            .as_ref()
            .and_then(|m| m.get("go:named-results"))
            .and_then(|m| m.get(owner.unwrap_or(&self.function.id)))
            .and_then(|s| s.as_array());
        self.result_types(owner)?
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                let symbol = names
                    .and_then(|items| items.get(i))
                    .and_then(|id| id.as_str())
                    .map(str::to_owned);
                let initial = match t.kind {
                    TypeKind::Integer | TypeKind::Float | TypeKind::Char => Some("0"),
                    TypeKind::Bool => Some("false"),
                    TypeKind::String => Some(""),
                    TypeKind::Sequence if t.fixed == Some(false) => Some("nil"),
                    TypeKind::Interface
                    | TypeKind::Reference
                    | TypeKind::Pointer
                    | TypeKind::Map
                    | TypeKind::Function => Some("nil"),
                    _ if self
                        .tables
                        .extension("go:channel-types")
                        .and_then(|m| m.get(&t.id))
                        .is_some() =>
                    {
                        Some("nil")
                    }
                    _ => None,
                };
                Some(ResultSlot {
                    ty: t.id.clone(),
                    symbol,
                    initial: initial.map(str::to_owned),
                })
            })
            .collect()
    }

    fn sources(&self, node: &Node, slots: &[ResultSlot]) -> Option<Vec<ResultSource>> {
        if node.children.is_empty() {
            return slots
                .iter()
                .map(|s| {
                    Some(ResultSource::Binding {
                        symbol: s.symbol.clone()?,
                    })
                })
                .collect();
        }
        let expr = self.body.node(node.children.first()?).ok()?;
        if expr.kind == NodeKind::Tuple && expr.original_kind == "ExprList" {
            if expr.children.len() != slots.len() {
                return None;
            }
            return Some(
                expr.children
                    .iter()
                    .map(|id| ResultSource::Expression {
                        node: id.clone(),
                        component: None,
                    })
                    .collect(),
            );
        }
        if slots.len() == 1 {
            return Some(vec![ResultSource::Expression {
                node: expr.id.clone(),
                component: None,
            }]);
        }
        if expr.kind != NodeKind::Call {
            return None;
        }
        let ty = self.tables.ty(expr.ty.as_deref()?).ok()?;
        if ty.kind != TypeKind::Tuple || ty.items.as_ref()?.len() != slots.len() {
            return None;
        }
        Some(
            (0..slots.len())
                .map(|component| ResultSource::Expression {
                    node: expr.id.clone(),
                    component: Some(component),
                })
                .collect(),
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultSlot {
    pub ty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResultSource {
    Binding {
        symbol: String,
    },
    Expression {
        node: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        component: Option<usize>,
    },
}

pub(crate) fn attach(tables: &Tables<'_>, function: &Function, derived: &mut Derived) {
    if tables.language() != "go"
        || !(function
            .extensions
            .as_ref()
            .is_some_and(|m| m.contains_key("go:named-results"))
            || function.nodes.iter().any(|n| {
                n.original_kind == "ReturnStmt"
                    || matches!(n.extension.as_deref(), Some("go:defer" | "go:go"))
            }))
    {
        return;
    }
    let Ok(body) = Body::new(function) else {
        return;
    };
    let frame = Frame {
        function,
        body,
        tables,
    };
    super::each_reached(function, derived, |graph, reached, own| {
        own.result_slots = frame.slots(graph.closure).unwrap_or_default();
        for id in &reached {
            let Ok(node) = frame.body.node(id) else {
                continue;
            };
            if let Some(call) = frame.invocation(node, graph, &reached) {
                own.scheduled_calls.insert(node.id.clone(), call);
            }
            if node.kind == NodeKind::Return
                && !own.result_slots.is_empty()
                && let Some(slots) = frame.sources(node, &own.result_slots)
            {
                own.return_sources.insert(node.id.clone(), slots);
            }
        }
    });
}
