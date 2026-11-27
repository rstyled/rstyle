//! Go leaves many sibling operand evaluations unsequenced. Build the language
//! partial order independently of the concrete order used by the emitted CFG.
use super::Derived;
use crate::cpg::model::{AccessMode, Function, Node, NodeKind, TypeKind};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvaluationOrder {
    pub events: BTreeMap<String, String>,
    /// Operand completion and lexical effect ordering, conditional on execution.
    pub must_precede: Vec<[String; 2]>,
    /// No relative order is required by the language between these operations.
    pub unsequenced: Vec<[String; 2]>,
}

struct Context<'a, 'b> {
    body: &'b Body<'a>,
    tables: &'b Tables<'a>,
    reads: HashSet<&'a str>,
}

// Expression events and their transitive predecessor sets, capped at 48
// interesting operations; larger expressions receive no partial summary.
struct Walk<'a, 'b, 'c> {
    ctx: &'c Context<'a, 'b>,
    reached: &'c HashSet<&'a str>,
    events: Vec<(&'a str, &'static str, u64)>,
    effect: Option<usize>,
    budget: usize,
}

impl<'a> Walk<'a, '_, '_> {
    fn event(&self, n: &Node, map_entry: bool) -> Option<(&'static str, bool)> {
        match n.kind {
            NodeKind::Tuple if map_entry && n.original_kind == "KeyValueExpr" => {
                Some(("map_store", false))
            }
            NodeKind::Call if n.original_kind == "CallExpr" => {
                let constant = self
                    .ctx
                    .tables
                    .extension("go:operand-modes")
                    .and_then(|m| m.get(&n.id))
                    .and_then(|v| v.get("constant"))
                    .and_then(|v| v.as_bool());
                (constant != Some(true)).then_some(("call", true))
            }
            NodeKind::Operator => match n.operator.as_deref()? {
                "go:receive" => Some(("receive", true)),
                "and" | "or" => Some(("logical", true)),
                "deref" => Some(("indirection", false)),
                _ => None,
            },
            NodeKind::IndexAccess => Some(("index", false)),
            NodeKind::FieldAccess => Some(("field", false)),
            NodeKind::Identifier if self.ctx.reads.contains(n.id.as_str()) => Some(("read", false)),
            _ => None,
        }
    }

    fn walk(&mut self, n: &'a Node, state: (bool, u64), depth: usize) -> Option<u64> {
        let (map_entry, guard) = state;
        self.budget = self.budget.checked_sub(1)?;
        if depth > 96 || self.events.len() >= 48 {
            return None;
        }
        // Elided operands and closure bodies don't participate when values are evaluated.
        if !self.reached.contains(n.id.as_str()) || n.kind == NodeKind::Closure {
            return Some(0);
        }
        let map = n.kind == NodeKind::Array
            && n.ty
                .as_deref()
                .and_then(|id| self.ctx.tables.ty(id).ok())
                .is_some_and(|t| t.kind == TypeKind::Map);
        let short =
            n.kind == NodeKind::Operator && matches!(n.operator.as_deref(), Some("and" | "or"));
        let mut predecessors = guard;
        for (i, c) in n.children.iter().enumerate() {
            let prior = if short && i == 1 { predecessors } else { guard };
            predecessors |= self.walk(self.ctx.body.node(c).ok()?, (map, prior), depth + 1)?;
        }
        let Some((kind, ordered)) = self.event(n, map_entry) else {
            return Some(predecessors);
        };
        let i = self.events.len();
        if i >= 48 {
            return None;
        }
        if ordered {
            if let Some(prior) = self.effect {
                predecessors |= (1 << prior) | self.events[prior].2;
            }
            self.effect = Some(i);
        }
        self.events.push((&n.id, kind, predecessors));
        Some(predecessors | (1 << i))
    }

    fn finish(self) -> Option<EvaluationOrder> {
        if self.events.len() < 2
            || !(self.effect.is_some() || self.events.iter().any(|e| e.1 == "map_store"))
        {
            return None;
        }
        let mut row = EvaluationOrder {
            events: BTreeMap::new(),
            must_precede: vec![],
            unsequenced: vec![],
        };
        for (i, (id, kind, ancestors)) in self.events.iter().enumerate() {
            row.events.insert((*id).into(), (*kind).into());
            for (j, (src, _, _)) in self.events[..i].iter().enumerate() {
                let pair = [(*src).into(), (*id).into()];
                if ancestors & (1 << j) != 0 {
                    row.must_precede.push(pair);
                } else {
                    row.unsequenced.push(pair);
                }
            }
        }
        Some(row)
    }
}

fn root<'a>(body: &Body<'a>, n: &Node) -> Option<&'a Node> {
    match n.kind {
        NodeKind::Return | NodeKind::LocalDeclaration | NodeKind::Assignment => {
            body.node(n.children.last()?).ok()
        }
        NodeKind::ExpressionStatement => {
            let v = body.child(n, 0).ok()?;
            (v.kind == NodeKind::Call).then_some(v)
        }
        _ => None,
    }
}

pub(crate) fn attach(tables: &Tables<'_>, f: &Function, derived: &mut Derived) {
    if tables.language() != "go"
        || !f.nodes.iter().any(|n| {
            matches!(
                n.original_kind.as_str(),
                "CallExpr" | "KeyValueExpr" | "UnaryExpr"
            )
        })
    {
        return;
    }
    let Ok(body) = Body::new(f) else { return };
    let ctx = Context {
        body: &body,
        tables,
        reads: f
            .accesses
            .iter()
            .filter(|a| a.mode != AccessMode::Write)
            .map(|a| a.node.as_str())
            .collect(),
    };
    super::each_reached(f, derived, |_, reached, own| {
        for n in &f.nodes {
            let Some(rhs) = root(&body, n) else {
                continue;
            };
            if !reached.contains(rhs.id.as_str()) || own.evaluation_orders.contains_key(&rhs.id) {
                continue;
            }
            let mut w = Walk {
                ctx: &ctx,
                reached: &reached,
                events: vec![],
                effect: None,
                budget: 512,
            };
            if w.walk(rhs, (false, 0), 0).is_some()
                && let Some(rec) = w.finish()
            {
                own.evaluation_orders.insert(rhs.id.clone(), rec);
            }
        }
    });
}
