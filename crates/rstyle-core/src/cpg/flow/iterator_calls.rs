//! For a locally resolved Go function-range operand, trace the iterator until
//! it yields, returns, or tries to yield again after the callback returned false.

use super::{Derived, Flow, Vertex};
use crate::cpg::model::{AccessMode, Dispatch, Function, Node, NodeKind, TypeKind};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Callback {
    pub operand: String,
    pub iterator: String,
    pub entry: String,
    pub initial: Destinations,
    pub yields: BTreeMap<String, Yield>,
    /// Caller sites that return true or false to the suspension point.
    pub feedback: BTreeMap<String, bool>,
    /// Loop-variable destinations, ordered as in the callback's signature.
    pub bindings: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Destinations {
    pub yields: BTreeSet<String>,
    pub exits: BTreeSet<String>,
    pub reentry_panics: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Yield {
    pub arguments: Vec<String>,
    pub on_true: Destinations,
    pub on_false: Destinations,
}

struct IteratorBody<'a> {
    function: &'a Function,
    graph: Flow<'a>,
    yields: HashMap<Vertex, &'a Node>,
    negated: HashMap<Vertex, Vertex>,
}

fn callback_parameter<'a>(tables: &Tables<'_>, function: &'a Function) -> Option<(&'a str, usize)> {
    let [cb] = &function.signature.parameters[..] else {
        return None;
    };
    let ty = tables.ty(&cb.ty).ok()?;
    let result = tables.ty(ty.result.as_deref()?).ok()?;
    let arity = ty.parameters.as_ref()?.len();
    if ty.kind != TypeKind::Function
        || result.kind != TypeKind::Bool
        || arity > 2
        || tables.ty(&function.signature.result).ok()?.kind != TypeKind::Unit
    {
        return None;
    }
    Some((&cb.symbol, arity))
}

impl<'a> IteratorBody<'a> {
    fn of(tables: &Tables<'_>, function: &'a Function) -> Option<Self> {
        let (symbol, arity) = callback_parameter(tables, function)?;
        let body = Body::new(function).ok()?;
        if body.symbol(symbol).ok()?.mutable {
            return None;
        }
        let graph = Flow::of(function)?;
        let reached = graph.reached();
        let mut uses: HashSet<_> = function
            .accesses
            .iter()
            .filter(|a| a.symbol == symbol && a.mode == AccessMode::Read)
            .map(|a| a.node.as_str())
            .collect();
        let mut yields = HashMap::new();
        for call in function
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Call && reached.contains(n.id.as_str()))
        {
            let Some(callee) = call.children.first() else {
                continue;
            };
            if body.reference(&call.id).and_then(|r| r.dispatch) == Some(Dispatch::FunctionValue)
                && call.children.len() == 1 + arity
                && uses.remove(callee.as_str())
            {
                yields.insert(graph.vertex(&call.id)?, call);
            }
        }
        // An escaped callback may run through an unmodeled alias.
        if !uses.is_empty()
            || yields.is_empty()
            || yields.len().saturating_mul(function.cfg.edges.len()) > 500_000
        {
            return None;
        }
        let negated = function
            .nodes
            .iter()
            .filter(|n| n.operator.as_deref() == Some("not") && n.children.len() == 1)
            .filter_map(|n| Some((graph.vertex(&n.children[0])?, graph.vertex(&n.id)?)))
            .collect();
        Some(Self {
            function,
            graph,
            yields,
            negated,
        })
    }

    fn successors(&self, node: Vertex, value: Option<(Vertex, bool)>, stack: &mut Vec<Vertex>) {
        let condition = value.and_then(|(call, truth)| {
            if call == node {
                Some(truth)
            } else if self.negated.get(&call) == Some(&node) {
                Some(!truth)
            } else {
                None
            }
        });
        for &(target, kind, _) in &self.graph.edges[node] {
            if matches!(
                (condition, kind),
                (Some(true), "false") | (Some(false), "true")
            ) {
                continue;
            }
            stack.push(target);
        }
    }

    fn destinations(&self, start: Vertex, result: Option<bool>) -> Destinations {
        let mut out = Destinations::default();
        let mut stack = Vec::new();
        let value = result.map(|r| (start, r));
        self.successors(start, value, &mut stack);
        let mut seen = HashSet::new();
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            let id = self.graph.id(node);
            if self.yields.contains_key(&node) {
                let targets = if result == Some(false) {
                    &mut out.reentry_panics
                } else {
                    &mut out.yields
                };
                targets.insert(id.to_owned());
            } else if self.graph.exits.contains(&node) {
                out.exits.insert(id.to_owned());
            } else {
                self.successors(node, value, &mut stack);
            }
        }
        out
    }

    fn schedule(&self, loop_node: &Node, body: &Body<'_>, graph: &Flow<'_>) -> Option<Callback> {
        let operand = body.child(loop_node, 1).ok()?;
        let mut yields = BTreeMap::new();
        for (&at, call) in &self.yields {
            yields.insert(
                call.id.clone(),
                Yield {
                    arguments: call.children[1..].to_vec(),
                    on_true: self.destinations(at, Some(true)),
                    on_false: self.destinations(at, Some(false)),
                },
            );
        }
        let pattern = body.child(loop_node, 0).ok()?;
        let bindings = match pattern.kind {
            NodeKind::PatternWildcard if pattern.original_kind == "RangeStmt.range" => Vec::new(),
            NodeKind::PatternTuple => pattern.children.clone(),
            _ => vec![pattern.id.clone()],
        };
        Some(Callback {
            iterator: self.function.id.clone(),
            operand: operand.id.clone(),
            entry: self.function.cfg.entry.clone(),
            initial: self.destinations(self.graph.entry(), None),
            yields,
            bindings,
            feedback: feedback(body, graph, loop_node)?,
        })
    }
}

fn feedback(body: &Body<'_>, graph: &Flow<'_>, loop_node: &Node) -> Option<BTreeMap<String, bool>> {
    let head = graph.vertex(&loop_node.id)?;
    let subtree = body.child(loop_node, 2).ok()?;
    let mut members = HashSet::new();
    let mut pending = vec![subtree];
    while let Some(node) = pending.pop() {
        if !members.insert(node.id.as_str()) || node.kind == NodeKind::Closure {
            continue;
        }
        pending.extend(body.children(node).ok()?);
    }
    let mut feedback = BTreeMap::new();
    for from in &members {
        let Some(v) = graph.vertex(from) else {
            continue;
        };
        for &(target, kind, role) in &graph.edges[v] {
            if target == head && role == Some("back_edge") {
                feedback.insert((*from).to_string(), true);
            }
            if matches!(kind, "return" | "break") && !members.contains(graph.id(target)) {
                feedback.insert((*from).to_string(), false);
            }
        }
    }
    Some(feedback)
}

pub(crate) fn attach(tables: &Tables<'_>, function: &Function, derived: &mut Derived) {
    if tables.language() != "go" {
        return;
    }
    let heads: Vec<_> = function
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::For && n.original_kind == "RangeStmt")
        .collect();
    if heads.is_empty() {
        return;
    }
    let Ok(body) = Body::new(function) else {
        return;
    };
    super::each_reached(function, derived, |graph, reached, own| {
        for head in &heads {
            if !reached.contains(head.id.as_str()) {
                continue;
            }
            let Some(callee) = body
                .child(head, 1)
                .ok()
                .and_then(|op| body.reference(&op.id))
                .and_then(|r| r.target.as_ref())
                .and_then(|t| t.id.as_deref())
                .and_then(|id| tables.function(id))
            else {
                continue;
            };
            if let Some(record) =
                IteratorBody::of(tables, callee).and_then(|it| it.schedule(head, &body, graph))
            {
                own.range_callbacks.insert(head.id.clone(), record);
            }
        }
    });
}
