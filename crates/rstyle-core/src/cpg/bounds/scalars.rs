//! Exact primitive values on straight-line paths from function entry, including a branch
//! whose condition is known. Values are recorded at their point of use, never by symbol
//! alone. At a loop only its once-evaluated source can use the incoming state. Anything
//! that might change a local outside this model, transfer control, or call user code
//! ends the walk. The ordinary bounds lowering still accounts for all execution costs.

mod address_words;
mod aggregates;
mod calls;
mod channels;
mod closures;
mod constants;
mod conversions;
mod counted;
mod floats;
mod frames;
mod interfaces;
mod maps;
mod method_instances;
mod operations;
mod pointers;
mod reflection;
mod storage;
mod unsafe_memory;

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
    Real(f64),
    Complex(f64, f64),
    Text(Vec<u8>),
    Tuple(Vec<Scalar>),
    ClosureValue(closures::Closure),
    NilFunction,
    StringView(storage::View),
    RawPointer(Option<pointers::Address>, Option<String>),
    AddressWord(address_words::Word),
    Sequence(Arc<[Scalar]>),
    Slice(storage::View),
    Map(Option<usize>),
    Channel(Option<usize>),
    Pointer(Option<pointers::Address>),
    ReflectedType(String),
    Callable(String),
    BoundMethod(String, Box<Scalar>),
    Interface(Option<Box<(String, Scalar)>>),
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
    heap: Vec<Scalar>,
    array_slots: BTreeMap<&'a str, usize>,
    environments: Vec<closures::Environment>,
    named_return: Option<&'a str>,
    inside_literal: bool,
    write_at: Option<&'a str>,
    round_at: Option<&'a str>,
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
            heap: Vec::new(),
            array_slots: BTreeMap::new(),
            environments: Vec::new(),
            named_return: None,
            inside_literal: false,
            write_at: None,
            round_at: None,
        }
    }
}

impl<'a> Walk<'_, '_, 'a> {
    fn enter(&mut self, node: &Node, depth: usize) -> Option<()> {
        self.remaining = self.remaining.checked_sub(1)?;
        let modifiers = node.modifiers.is_empty()
            || (node.kind == NodeKind::Reference && node.modifiers == ["mutable"]);
        (depth <= 48 && !self.facts.adjusted_of(node) && modifiers).then_some(())
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
        let ty = self.facts.ty(place)?;
        match (&value, ty.kind) {
            (Scalar::ReflectedType(_), TypeKind::Interface)
                if self.standard_type(ty, &self.profile.reflect.type_object) =>
            {
                Some(value)
            }
            (Scalar::AddressWord(_), TypeKind::Integer) if self.uintptr(ty) => Some(value),
            (Scalar::Int(n), TypeKind::Integer)
                if operations::narrow(*n, self.facts.integer(Some(ty))?) == *n =>
            {
                Some(value)
            }
            (Scalar::Int(_), TypeKind::Opaque) if self.unbounded_integer(place) => Some(value),
            (Scalar::Nil, TypeKind::Sequence) if ty.fixed == Some(false) => {
                Some(Scalar::Slice(storage::View::nil()))
            }
            (Scalar::Real(_), TypeKind::Float) => Some(value),
            (Scalar::Complex(_, _), TypeKind::Opaque) if self.complex_width(ty).is_some() => {
                Some(value)
            }
            (Scalar::Nil, TypeKind::Map) => Some(Scalar::Map(None)),
            (Scalar::Nil, TypeKind::Function) => Some(Scalar::NilFunction),
            (Scalar::ClosureValue(_) | Scalar::NilFunction, TypeKind::Function)
            | (Scalar::Tuple(_), TypeKind::Tuple)
            | (Scalar::Interface(_), TypeKind::Interface) => Some(value),
            (Scalar::Map(_), TypeKind::Map)
            | (Scalar::Pointer(_), TypeKind::Reference)
            | (Scalar::Callable(_) | Scalar::BoundMethod(_, _), TypeKind::Function) => Some(value),
            (Scalar::RawPointer(_, _), TypeKind::Pointer) if self.raw_type(ty) => Some(value),
            (Scalar::Nil, TypeKind::Pointer) if self.raw_type(ty) => {
                Some(Scalar::RawPointer(None, None))
            }
            (Scalar::Nil, TypeKind::Reference) => Some(Scalar::Pointer(None)),
            (Scalar::Channel(_), TypeKind::Opaque) if self.channel_element(ty).is_some() => {
                Some(value)
            }
            (Scalar::Nil, TypeKind::Opaque) if self.channel_element(ty).is_some() => {
                Some(Scalar::Channel(None))
            }
            (Scalar::Slice(_), TypeKind::Sequence) if ty.fixed == Some(false) => Some(value),
            (Scalar::Text(_) | Scalar::StringView(_), TypeKind::String)
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
            (NodeKind::ItemDeclaration, 0) => Some(()),
            (NodeKind::Block, _) if node.label.is_none() => {
                for id in &node.children {
                    self.statement(self.facts.body.node(id).ok()?, depth + 1)?;
                }
                Some(())
            }
            (NodeKind::ExpressionStatement, 1) => self.statement(self.child(node, 0)?, depth + 1),
            (
                NodeKind::LocalDeclaration | NodeKind::CompoundAssignment | NodeKind::Assignment,
                _,
            ) => self.assignment_statement(node, depth),
            (NodeKind::If, 2 | 3) => self.branch(node, depth),
            (NodeKind::While, 2) => self.counted_loop(node, depth),
            (NodeKind::For, 3) => {
                let operand = self.child(node, 1)?;
                self.write_at = Some(&operand.id);
                let source = self.value(operand, depth + 1)?;
                match source {
                    Scalar::Channel(Some(slot)) => self.channel_range(node, slot, depth + 1),
                    Scalar::Map(slot) => self.singleton_range(node, slot, depth + 1),
                    _ => None,
                }
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
            (NodeKind::Return, 0 | 1) => {
                self.returned = self.return_value(node, depth);
                None
            }
            _ => None,
        }
    }

    fn return_value(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        if let Some(id) = node.children.first() {
            self.write_at = Some(id);
            return self.value(self.child(node, 0)?, depth + 1);
        }
        match self.named_return {
            Some(symbol) => self.local_value(symbol),
            None if !self.inside_literal
                && self
                    .facts
                    .type_of(&self.facts.body.function.signature.result)?
                    .kind
                    == TypeKind::Unit =>
            {
                Some(Scalar::Nil)
            }
            _ => None,
        }
    }

    fn assignment_statement(&mut self, node: &'a Node, depth: usize) -> Option<()> {
        match (node.kind, node.children.len()) {
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
                    (self.child(node, 0)?, self.child(node, 2)?),
                    node.kind,
                    depth + 1,
                )
            }
            (NodeKind::CompoundAssignment, 2) if self.facts.reference(node).is_none() => {
                self.compound(node, depth)
            }
            (NodeKind::LocalDeclaration | NodeKind::Assignment, 2) => {
                let place = self.child(node, 0)?;
                let rhs = self.child(node, 1)?;
                self.assign((place, rhs), node.kind, depth + 1)?;
                Some(())
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
                NodeKind::Identifier | NodeKind::Operator | NodeKind::Cast | NodeKind::Call
            )
            && self.constant_tree(node, depth)
            && let Some(known) = self.literal(node, &fact.value)
            && let Some(known) = self.for_storage(node, known)
        {
            self.values.insert(node.id.as_str(), known.clone());
            return Some(known);
        }
        if let Some(known) = self.complex_constant(node)
            && self.constant_tree(node, depth)
        {
            self.values.insert(&node.id, known.clone());
            return Some(known);
        }
        let raw = self.expression(node, depth)?;
        self.record_result(node, raw)
    }

    fn expression(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        Some(match (node.kind, node.children.len()) {
            (NodeKind::Literal, 0) => self.literal(node, node.value.as_ref()?)?,
            (NodeKind::Array, _) if self.facts.ty(node)?.kind == TypeKind::Map => {
                self.map_literal(node, depth + 1)?
            }
            (NodeKind::Array, _) => self.array(node, depth + 1)?,
            (NodeKind::Extension, 1)
                if node.extension.as_deref() == Some(&self.profile.method_value) =>
            {
                self.method_value(node, depth)?
            }
            (NodeKind::Extension, _)
                if node.extension.as_deref() == Some(self.profile.slice_expression.as_str()) =>
            {
                self.slice(node, depth + 1)?
            }
            (NodeKind::Tuple, _) => self.tuple(node, depth + 1)?,
            (NodeKind::Closure, _) if self.facts.ty(node)?.kind == TypeKind::Function => {
                self.closure_value(node)
            }
            (NodeKind::RecordConstruction, _) => self.record(node, depth + 1)?,
            (NodeKind::FieldAccess, 1) if self.facts.reference(node).is_none() => {
                self.field_value(node, depth + 1)?
            }
            (NodeKind::Reference, 1)
                if self.facts.ty(node)?.kind == TypeKind::Reference
                    && self.facts.ty(node)?.target == self.child(node, 0)?.ty =>
            {
                Scalar::Pointer(Some(self.address(self.child(node, 0)?, depth + 1)?))
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
                self.operator_value(node, depth)?
            }
            (NodeKind::IndexAccess, 2) if self.facts.reference(node).is_none() => {
                self.index_value(node, depth)?
            }
            (NodeKind::Call, _) => self.call(node, depth + 1)?,
            _ => return None,
        })
    }

    fn record_result(&mut self, node: &'a Node, raw: Scalar) -> Option<Scalar> {
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
            (v @ Scalar::Real(_), TypeKind::Opaque)
                if self.facts.ty(node)?.key.as_deref() == Some(&self.profile.untyped_float)
                    && !runtime =>
            {
                v
            }
            (
                v @ (Scalar::Int(_) | Scalar::Real(_) | Scalar::Complex(_, _)),
                TypeKind::Float | TypeKind::Opaque,
            ) => self.convert_numeric(&v, self.facts.ty(node)?)?,
            (v @ Scalar::Nil, _) => v,
            (v, _) if !matches!(v, Scalar::Int(_) | Scalar::Real(_) | Scalar::Complex(_, _)) => {
                self.for_storage(node, v)?
            }
            _ => return None,
        };
        if runtime {
            self.runtime.insert(node.id.as_str());
        }
        self.values.insert(node.id.as_str(), result.clone());
        Some(result)
    }

    fn local_value(&self, symbol: &str) -> Option<Scalar> {
        if let Some(slot) = self.array_slots.get(symbol) {
            return Some(Scalar::Sequence(self.buffers.get(*slot)?.clone()));
        }
        self.locals.get(symbol).cloned()
    }

    fn identifier(&self, node: &Node) -> Option<Scalar> {
        if self.facts.reference(node)?.status != RefStatus::Resolved {
            return None;
        }
        if let Some(symbol) = self.facts.local(node) {
            return self.local_value(symbol);
        }
        let (_, def) = self.facts.resolved(node)?;
        if def.standard && def.operation.as_deref() == Some(&self.profile.nil) {
            return Some(Scalar::Nil);
        }
        let ty = self.facts.ty(node)?;
        if matches!(def.kind, DefinitionKind::Method | DefinitionKind::Function)
            && ty.kind == TypeKind::Function
            && ty.definition.as_ref() == Some(&def.id)
        {
            return Some(Scalar::Callable(def.id.clone()));
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
    fn compound(&mut self, node: &'a Node, depth: usize) -> Option<()> {
        let place = self.child(node, 0)?;
        let addr = self.address(place, depth + 1)?;
        let before = self.pointed(&addr)?;
        let rhs = self.value(self.child(node, 1)?, depth + 1)?;
        let ty = self.facts.integer(self.facts.ty(place))?;
        let Scalar::Int(v) = operations::apply(
            self.profile,
            node.operator.as_deref()?,
            (before, Some(rhs)),
            (Some(ty), true),
        )?
        else {
            return None;
        };
        self.write_address(&addr, Scalar::Int(operations::narrow(v, ty)))
    }

    fn branch(&mut self, node: &'a Node, depth: usize) -> Option<()> {
        let Scalar::Bool(test) = self.value(self.child(node, 0)?, depth + 1)? else {
            return None;
        };
        if !test && node.children.len() == 2 {
            return Some(());
        }
        self.statement(self.child(node, if test { 1 } else { 2 })?, depth + 1)
    }

    fn operator_value(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        let first = self.value(self.child(node, 0)?, depth + 1)?;
        let op = node.operator.as_deref()?;
        if op == "deref" && node.children.len() == 1 {
            let Scalar::Pointer(Some(addr)) = first else {
                return None;
            };
            return self.pointed(&addr);
        }
        // A skipped operand cannot execute or contribute an expression-site fact.
        if matches!(
            (&first, op),
            (Scalar::Bool(false), "and") | (Scalar::Bool(true), "or")
        ) && node.children.len() == 2
            && self.facts.ty(node)?.kind == TypeKind::Bool
        {
            return Some(first);
        }
        let second = match node.children.len() {
            2 => Some(self.value(self.child(node, 1)?, depth + 1)?),
            _ => None,
        };
        self.binary_value(node, (first, second), depth)
    }

    fn binary_value(
        &mut self,
        node: &'a Node,
        (first, second): (Scalar, Option<Scalar>),
        depth: usize,
    ) -> Option<Scalar> {
        let op = node.operator.as_deref()?;
        if let (Scalar::ReflectedType(a), Some(Scalar::ReflectedType(b))) = (&first, &second)
            && matches!(op, "eq" | "ne")
        {
            return Some(Scalar::Bool(
                self.same_reflected_type(a, b)? == (op == "eq"),
            ));
        }
        if matches!(first, Scalar::AddressWord(_)) || matches!(second, Some(Scalar::AddressWord(_)))
        {
            return self.word_operation(op, first, second?);
        }
        if matches!(first, Scalar::StringView(_)) || matches!(second, Some(Scalar::StringView(_))) {
            let a = self.read_string(first)?;
            let b = self.read_string(second?)?;
            return operations::apply(self.profile, op, (a, Some(b)), (None, false));
        }
        if matches!(op, "eq" | "ne")
            && second.is_some()
            && (matches!(
                first,
                Scalar::Interface(_) | Scalar::Sequence(_) | Scalar::Record(_)
            ) || matches!(second, Some(Scalar::Interface(_)))
                || matches!(
                    (&first, &second),
                    (Scalar::Pointer(_), Some(Scalar::Pointer(_)))
                ))
        {
            return self.equality(node, (first, second?), depth);
        }
        if matches!(first, Scalar::Real(_) | Scalar::Complex(_, _)) {
            return self.floating_op(node, op, (first, second));
        }
        operations::apply(
            self.profile,
            op,
            (first, second),
            (self.facts.integer(self.facts.ty(node)), self.runtime(node)),
        )
    }

    fn index_value(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        let base = self.value(self.child(node, 0)?, depth + 1)?;
        if let Scalar::Map(slot) = base {
            let map = self.child(node, 0)?;
            let key = self.map_key(map, self.child(node, 1)?, depth + 1)?;
            return Some(self.lookup(map, slot, key)?.0);
        }
        let key = self.value(self.child(node, 1)?, depth + 1)?;
        let Scalar::Int(i) = key else {
            return None;
        };
        let i = usize::try_from(i).ok()?;
        Some(match base {
            Scalar::Pointer(Some(addr)) => {
                let elem = self
                    .facts
                    .type_of(self.facts.ty(self.child(node, 0)?)?.target.as_deref()?)?
                    .element
                    .as_deref()?;
                if Some(elem) != node.ty.as_deref() {
                    return None;
                }
                let Scalar::Sequence(items) = self.pointed(&addr)? else {
                    return None;
                };
                items.get(i)?.clone()
            }
            Scalar::Text(s) => Scalar::Int(i128::from(*s.get(i)?)),
            Scalar::Sequence(items) => items.get(i)?.clone(),
            Scalar::Slice(view) | Scalar::StringView(view) => self.element(view, i)?.clone(),
            _ => return None,
        })
    }
}
