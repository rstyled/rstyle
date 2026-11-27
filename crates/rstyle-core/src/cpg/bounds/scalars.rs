//! Exact primitive values on straight-line paths from function entry, including a branch
//! whose condition is known. Values are recorded at their point of use, never by symbol
//! alone. At a loop only its once-evaluated source can use the incoming state. Anything
//! that might change a local outside this model, transfer control, or call user code
//! ends the walk. The ordinary bounds lowering still accounts for all execution costs.

mod aggregates;
mod calls;
mod channels;
mod constants;
mod conversions;
mod maps;
mod operations;
mod storage;

use super::super::model::{DefinitionKind, Literal, Node, NodeKind, RefStatus, TypeKind};
use super::facts::Facts;
use crate::runtime_bounds::Arithmetic;
use crate::runtime_bounds::contracts::{Contracts, ScalarProfile};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Scalar {
    Int(i128),
    Bool(bool),
    Text(Vec<u8>),
    Sequence(Arc<[Scalar]>),
    Slice(storage::View),
    Map(Option<usize>),
    Channel(Option<usize>),
    Nil,
    Record(Arc<BTreeMap<String, Scalar>>),
}

type Values<'a> = BTreeMap<&'a str, Scalar>;

pub(super) fn known<'a>(facts: &Facts<'_, 'a>, arithmetic: Arithmetic) -> Values<'a> {
    let Some(profile) = Contracts::embedded().scalar(facts.tables.language()) else {
        return Values::new();
    };
    if arithmetic != Arithmetic::Wrapping || !matches!(facts.pointer_bits, 8 | 16 | 32 | 64) {
        return Values::new();
    }
    let mut walk = Walk::new(facts, profile, 2048);
    if let Ok(root) = facts.body.root() {
        walk.statement(root, 0);
    }
    walk.values
}

struct Walk<'b, 'f, 'a> {
    facts: &'b Facts<'f, 'a>,
    profile: &'b ScalarProfile,
    values: Values<'a>,
    locals: Values<'a>,
    runtime: BTreeSet<&'a str>,
    constants: BTreeMap<&'a str, &'a crate::cpg::model::Value>,
    bindings: BTreeMap<&'a str, &'a str>,
    remaining: usize,
    returned: Option<Scalar>,
    buffers: Vec<Arc<[Scalar]>>,
    maps: maps::Maps,
    channels: Vec<channels::Channel>,
    array_slots: BTreeMap<&'a str, usize>,
    write_at: Option<&'a str>,
}

impl<'b, 'f, 'a> Walk<'b, 'f, 'a> {
    fn new(facts: &'b Facts<'f, 'a>, profile: &'b ScalarProfile, remaining: usize) -> Self {
        Self {
            facts,
            profile,
            values: Values::new(),
            locals: Values::new(),
            runtime: BTreeSet::new(),
            constants: facts
                .body
                .function
                .values
                .iter()
                .filter_map(|v| Some((v.node.as_deref()?, v)))
                .collect(),
            bindings: facts
                .body
                .function
                .symbols
                .iter()
                .filter(|s| s.kind == "local")
                .filter_map(|s| Some((s.node.as_deref()?, s.id.as_str())))
                .collect(),
            remaining,
            returned: None,
            buffers: Vec::new(),
            maps: Vec::new(),
            channels: Vec::new(),
            array_slots: BTreeMap::new(),
            write_at: None,
        }
    }
}

impl<'a> Walk<'_, '_, 'a> {
    fn enter(&mut self, node: &Node, depth: usize) -> Option<()> {
        self.remaining = self.remaining.checked_sub(1)?;
        (depth <= 48 && !self.facts.adjusted_of(node) && node.modifiers.is_empty()).then_some(())
    }

    fn child(&self, node: &Node, at: usize) -> Option<&'a Node> {
        self.facts.body.child(node, at).ok()
    }

    fn unbounded_integer(&self, node: &Node) -> bool {
        self.facts
            .ty(node)
            .and_then(|ty| ty.key.as_ref())
            .is_some_and(|key| self.profile.exact_integer_types.contains(key))
    }

    fn for_storage(&self, place: &Node, value: Scalar) -> Option<Scalar> {
        match (&value, self.facts.ty(place)?.kind) {
            (Scalar::Int(n), TypeKind::Integer)
                if operations::narrow(*n, self.facts.integer(self.facts.ty(place))?) == *n =>
            {
                Some(value)
            }
            (Scalar::Int(_), TypeKind::Opaque) if self.unbounded_integer(place) => Some(value),
            (Scalar::Nil, TypeKind::Sequence) if self.facts.ty(place)?.fixed == Some(false) => {
                Some(Scalar::Slice(storage::View::nil()))
            }
            (Scalar::Nil, TypeKind::Map) => Some(Scalar::Map(None)),
            (Scalar::Map(_), TypeKind::Map) => Some(value),
            (Scalar::Channel(_), TypeKind::Opaque)
                if self.channel_element(self.facts.ty(place)?).is_some() =>
            {
                Some(value)
            }
            (Scalar::Nil, TypeKind::Opaque)
                if self.channel_element(self.facts.ty(place)?).is_some() =>
            {
                Some(Scalar::Channel(None))
            }
            (Scalar::Slice(_), TypeKind::Sequence)
                if self.facts.ty(place)?.fixed == Some(false) =>
            {
                Some(value)
            }
            (Scalar::Text(_), TypeKind::String)
            | (Scalar::Bool(_), TypeKind::Bool)
            | (Scalar::Sequence(_), TypeKind::Sequence)
            | (Scalar::Record(_), TypeKind::Record) => Some(value),
            _ => None,
        }
    }

    fn runtime(&self, node: &Node) -> bool {
        self.facts.local(node).is_some()
            || node
                .children
                .iter()
                .any(|id| self.runtime.contains(id.as_str()))
    }

    fn statement(&mut self, node: &'a Node, depth: usize) -> Option<()> {
        self.enter(node, depth)?;
        match (node.kind, node.children.len()) {
            (NodeKind::Block, _) if node.label.is_none() => {
                for id in &node.children {
                    self.statement(self.facts.body.node(id).ok()?, depth + 1)?;
                }
                Some(())
            }
            (NodeKind::ExpressionStatement, 1) => self.statement(self.child(node, 0)?, depth + 1),
            (NodeKind::LocalDeclaration, 2)
                if self.child(node, 1)?.kind == NodeKind::TypeAnnotation =>
            {
                let place = self.child(node, 0)?;
                let value = self.zero(self.facts.ty(place)?)?;
                self.store(self.binding(place)?, value)?;
                Some(())
            }
            (NodeKind::LocalDeclaration, 3)
                if self.child(node, 1)?.kind == NodeKind::TypeAnnotation =>
            {
                self.assign(
                    self.child(node, 0)?,
                    self.child(node, 2)?,
                    node.kind,
                    depth + 1,
                )
            }
            (NodeKind::CompoundAssignment, 2) if self.facts.reference(node).is_none() => {
                let place = self.child(node, 0)?;
                let symbol = self.facts.local(place)?;
                let before = self.locals.get(symbol)?.clone();
                let rhs = self.value(self.child(node, 1)?, depth + 1)?;
                let ty = self.facts.integer(self.facts.ty(place))?;
                let Scalar::Int(v) = operations::apply(
                    self.profile,
                    node.operator.as_deref()?,
                    before,
                    Some(rhs),
                    Some(ty),
                    true,
                )?
                else {
                    return None;
                };
                self.locals
                    .insert(symbol, Scalar::Int(operations::narrow(v, ty)));
                Some(())
            }
            (NodeKind::LocalDeclaration | NodeKind::Assignment, 2) => {
                let place = self.child(node, 0)?;
                let rhs = self.child(node, 1)?;
                self.assign(place, rhs, node.kind, depth + 1)?;
                Some(())
            }
            (NodeKind::If, 2 | 3) => {
                let Scalar::Bool(test) = self.value(self.child(node, 0)?, depth + 1)? else {
                    return None;
                };
                if !test && node.children.len() == 2 {
                    return Some(());
                }
                self.statement(self.child(node, if test { 1 } else { 2 })?, depth + 1)
            }
            (NodeKind::For, 3) => {
                self.value(self.child(node, 1)?, depth + 1);
                None
            }
            (NodeKind::Call, _) => {
                self.write_at = Some(&node.id);
                self.call(node, depth + 1)?;
                Some(())
            }
            (NodeKind::Operator, 2)
                if node.operator.as_deref() == Some(&self.profile.send)
                    && self.facts.reference(node).is_none() =>
            {
                self.send(node, depth + 1)
            }
            (NodeKind::Operator, 1)
                if node.operator.as_deref() == Some(&self.profile.receive)
                    && self.facts.reference(node).is_none() =>
            {
                self.receive(node, depth + 1).map(|_| ())
            }
            (NodeKind::Return, 1) => {
                self.write_at = Some(&self.child(node, 0)?.id);
                self.returned = self.value(self.child(node, 0)?, depth + 1);
                None
            }
            _ => None,
        }
    }

    fn value(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        self.enter(node, depth)?;
        // Typed compiler constants may have arbitrarily precise operands. Consume their
        // normalized result only for expressions that cannot run arbitrary code.
        if let Some(fact) = self.constants.get(node.id.as_str()).copied()
            && fact.ty == node.ty
            && matches!(
                node.kind,
                NodeKind::Identifier | NodeKind::Operator | NodeKind::Cast
            )
            && self.constant_tree(node, depth)
            && let Some(known) = self.literal(node, &fact.value)
            && self.for_storage(node, known.clone()).is_some()
        {
            self.values.insert(node.id.as_str(), known.clone());
            return Some(known);
        }
        let raw = match (node.kind, node.children.len()) {
            (NodeKind::Literal, 0) => self.literal(node, node.value.as_ref()?)?,
            (NodeKind::Array, _) if self.facts.ty(node)?.kind == TypeKind::Map => {
                self.map_literal(node, depth + 1)?
            }
            (NodeKind::Array, _) => self.array(node, depth + 1)?,
            (NodeKind::Extension, _)
                if node.extension.as_deref() == Some(self.profile.slice_expression.as_str()) =>
            {
                self.slice(node, depth + 1)?
            }
            (NodeKind::RecordConstruction, _) => self.record(node, depth + 1)?,
            (NodeKind::FieldAccess, 1) if self.facts.reference(node).is_none() => {
                let Scalar::Record(fields) = self.value(self.child(node, 0)?, depth + 1)? else {
                    return None;
                };
                fields.get(node.member.as_deref()?)?.clone()
            }
            (NodeKind::Identifier, 0) => self.identifier(node)?,
            (NodeKind::Cast, 1) if self.facts.reference(node).is_none() => {
                self.convert(node, depth + 1)?
            }
            (NodeKind::Operator, 1)
                if node.operator.as_deref() == Some(&self.profile.receive)
                    && self.facts.reference(node).is_none() =>
            {
                self.receive(node, depth + 1)?.0
            }
            (NodeKind::Operator, 1 | 2) if self.facts.reference(node).is_none() => {
                let first = self.value(self.child(node, 0)?, depth + 1)?;
                let op = node.operator.as_deref()?;
                // A skipped operand cannot execute or contribute an expression-site fact.
                if matches!(
                    (&first, op),
                    (Scalar::Bool(false), "and") | (Scalar::Bool(true), "or")
                ) && node.children.len() == 2
                    && self.facts.ty(node)?.kind == TypeKind::Bool
                {
                    self.values.insert(&node.id, first.clone());
                    return Some(first);
                }
                let second = match node.children.len() {
                    2 => Some(self.value(self.child(node, 1)?, depth + 1)?),
                    _ => None,
                };
                operations::apply(
                    self.profile,
                    op,
                    first,
                    second,
                    self.facts.integer(self.facts.ty(node)),
                    self.runtime(node),
                )?
            }
            (NodeKind::IndexAccess, 2) if self.facts.reference(node).is_none() => {
                let base = self.value(self.child(node, 0)?, depth + 1)?;
                let key = self.value(self.child(node, 1)?, depth + 1)?;
                match base {
                    Scalar::Map(slot) => {
                        self.lookup(self.child(node, 0)?, slot, maps::Key::of(key)?)?
                            .0
                    }
                    base => {
                        let Scalar::Int(i) = key else {
                            return None;
                        };
                        let i = usize::try_from(i).ok()?;
                        match base {
                            Scalar::Text(s) => Scalar::Int(i128::from(*s.get(i)?)),
                            Scalar::Sequence(items) => items.get(i)?.clone(),
                            Scalar::Slice(view) => self.element(view, i)?.clone(),
                            _ => return None,
                        }
                    }
                }
            }
            (NodeKind::Call, _) => self.call(node, depth + 1)?,
            _ => return None,
        };
        let runtime = self.runtime(node);
        let result = match (raw, self.facts.ty(node)?.kind) {
            (Scalar::Int(n), TypeKind::Integer) => {
                let ty = self.facts.integer(self.facts.ty(node))?;
                let k = if runtime || node.kind == NodeKind::Cast {
                    operations::narrow(n, ty)
                } else {
                    n
                };
                Scalar::Int(k)
            }
            (v @ Scalar::Int(_), TypeKind::Opaque) if !runtime && self.unbounded_integer(node) => v,
            (v @ Scalar::Nil, _) => v,
            (v @ Scalar::Map(_), TypeKind::Map) => v,
            (v @ Scalar::Channel(_), TypeKind::Opaque)
                if self.channel_element(self.facts.ty(node)?).is_some() =>
            {
                v
            }
            (v @ Scalar::Slice(_), TypeKind::Sequence)
                if self.facts.ty(node)?.fixed == Some(false) =>
            {
                v
            }
            (v @ Scalar::Bool(_), TypeKind::Bool)
            | (v @ Scalar::Text(_), TypeKind::String)
            | (v @ Scalar::Sequence(_), TypeKind::Sequence)
            | (v @ Scalar::Record(_), TypeKind::Record) => v,
            _ => return None,
        };
        if runtime {
            self.runtime.insert(node.id.as_str());
        }
        self.values.insert(node.id.as_str(), result.clone());
        Some(result)
    }

    fn identifier(&self, node: &Node) -> Option<Scalar> {
        if self.facts.reference(node)?.status != RefStatus::Resolved {
            return None;
        }
        if let Some(symbol) = self.facts.local(node) {
            if let Some(slot) = self.array_slots.get(symbol) {
                return Some(Scalar::Sequence(self.buffers.get(*slot)?.clone()));
            }
            return self.locals.get(symbol).cloned();
        }
        let (_, def) = self.facts.resolved(node)?;
        if def.standard && def.operation.as_deref() == Some(&self.profile.nil) {
            return Some(Scalar::Nil);
        }
        if !def.standard || def.kind != DefinitionKind::Constant {
            return None;
        }
        let op = def.operation.as_deref()?;
        if op == self.profile.true_constant {
            return Some(Scalar::Bool(true));
        }
        (op == self.profile.false_constant).then_some(Scalar::Bool(false))
    }
}
