//! Immutable standard-library extents, identified by resolved types and by the extent
//! roles their methods have in the library contracts.

use super::super::model::{
    DefinitionKind, Dispatch, Node, NodeKind, RefStatus, TargetKind, Type, TypeKind,
};
use super::Collector;
use super::allocation::named_function;
use super::facts::{Facts, is_shared_reference, shared_target};
use crate::runtime_bounds::contracts::{Contracts, ExtentRole, Runs};
use crate::runtime_bounds::{Field, Value};

#[derive(Clone)]
pub struct Place<'a> {
    pub root: &'a str,
    pub fields: Vec<usize>,
    /// Named field path, for composing formal-rooted lengths across calls.
    pub path: Vec<Field>,
    pub label: String,
}

impl Place<'_> {
    pub fn key(&self) -> String {
        format!("{}:{:?}", self.root, self.fields)
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.root == other.root
            && (self.fields.starts_with(&other.fields) || other.fields.starts_with(&self.fields))
    }
}

impl<'a> Collector<'_, '_, 'a> {
    pub(super) fn extent_value(&self, expression: &'a Node) -> Result<Option<Value>, String> {
        let Some(receiver) = extent_receiver(self.facts, expression)? else {
            return Ok(None);
        };
        self.sequence_extent(receiver)
    }

    /// A compiler-known fixed length, an exact native-allocation relation, or the length of
    /// a selected stable place. Requiring a place also keeps length reads from hiding
    /// evaluation of a computed receiver.
    pub(super) fn sequence_extent(&self, receiver: &'a Node) -> Result<Option<Value>, String> {
        let Some(place) = self.place(receiver)? else {
            return Ok(None);
        };
        if let Some(length) = self
            .facts
            .ty(receiver)
            .and_then(|ty| fixed_length(self.facts, ty))
        {
            let representable =
                self.facts.pointer_bits == 64 || length < (1u64 << self.facts.pointer_bits);
            return Ok(representable.then_some(Value::Const(length)));
        }
        if let Some(length) = self.allocation_extent(receiver)? {
            return Ok(Some(length));
        }

        Ok(self
            .extents
            .get(&place.key())
            .map(|label| Value::Input(label.clone())))
    }

    pub(super) fn place(&self, expression: &'a Node) -> Result<Option<Place<'a>>, String> {
        place(self.facts, expression, &self.numbers)
    }
}

pub(super) fn is_vec(facts: &Facts<'_, '_>, ty: &Type) -> bool {
    facts.type_key(ty) == Some("Vec") && facts.standard_definition(ty.definition.as_deref())
}

/// The measured operand of a resolved standard length operation.
pub(super) fn extent_receiver<'a>(
    facts: &Facts<'_, 'a>,
    expression: &'a Node,
) -> Result<Option<&'a Node>, String> {
    if !is_method(facts, expression) {
        return Ok(ordinary_length(facts, expression));
    }
    if expression.children.len() != 1 {
        return Ok(None);
    }
    if !has_role(facts, expression, ExtentRole::Length) {
        return Ok(None);
    }
    facts.body.child(expression, 0).map(Some)
}

/// A native one-argument function keeps its callee and operand evaluation in the IR.
pub(super) fn ordinary_length<'a>(facts: &Facts<'_, 'a>, call: &Node) -> Option<&'a Node> {
    let reference = facts.reference(call)?;
    let definition = facts.declared(reference)?;
    let role = definition
        .operation
        .as_deref()
        .and_then(|key| Contracts::embedded().extent(key))?;
    if call.kind != NodeKind::Call
        || call.children.len() != 2
        || facts.adjusted_of(call)
        || reference.status != RefStatus::Resolved
        || reference.dispatch != Some(Dispatch::Builtin)
        || reference.arguments.is_some()
        || reference.target.as_ref()?.kind != TargetKind::Definition
        || !definition.standard
        || definition.kind != DefinitionKind::Function
        || role.role != ExtentRole::Length
    {
        return None;
    }
    let callee = facts.body.child(call, 0).ok()?;
    let named = named_function(facts, callee, &definition.id);
    let receiver = facts.body.child(call, 1).ok()?;
    let native = facts
        .ty(receiver)
        .is_some_and(|ty| ty.kind == TypeKind::Sequence);
    (named && native && facts.integer(facts.ty(call)).is_some()).then_some(receiver)
}

pub(super) fn is_method(facts: &Facts<'_, '_>, call: &Node) -> bool {
    call.kind == NodeKind::Call
        && facts.reference(call).is_some_and(|reference| {
            matches!(
                reference.dispatch,
                Some(
                    super::super::model::Dispatch::Method | super::super::model::Dispatch::Dynamic
                )
            )
        })
}

/// Whether the resolved method is an inherent standard member with this extent role.
/// Interface members are excluded: their implementation, not the contract, decides.
pub(super) fn has_role(facts: &Facts<'_, '_>, call: &Node, role: ExtentRole) -> bool {
    let Some(reference) = facts.reference(call) else {
        return false;
    };
    let Some(method) = facts.declared(reference) else {
        return false;
    };

    // A method with no callable facts, as Python's are, is still its class's own.
    let owned = method
        .callable
        .as_ref()
        .map_or(method.kind == DefinitionKind::Method, |c| {
            c.owner_type.is_some()
        });
    let inherent =
        method.standard && method.implements.is_none() && !facts.interface_member(method) && owned;
    let key = inherent.then_some(method.operation.as_deref()).flatten();
    key.and_then(|key| Contracts::embedded().extent(key))
        .is_some_and(|extent| extent.role == role && receives(facts, call, &extent.receiver))
}

/// Whether a call's receiver has one of the type kinds a role names, when it names any.
fn receives(facts: &Facts<'_, '_>, call: &Node, kinds: &[String]) -> bool {
    if kinds.is_empty() {
        return true;
    }
    let receiver = call
        .children
        .first()
        .and_then(|id| facts.body.node(id).ok());
    let kind = receiver
        .and_then(|receiver| facts.ty(receiver))
        .map(|ty| ty.kind);
    kind.is_some_and(|kind| kinds.iter().any(|name| named(kind, name)))
}

/// Whether a type kind has this name in the profile.
fn named(kind: TypeKind, name: &str) -> bool {
    serde_json::from_value::<TypeKind>(serde_json::Value::String(name.into()))
        .is_ok_and(|named| named == kind)
}

/// Which arguments are the callables a standard function or member runs at most one of, once
/// for the call, by its callable role in the library contracts: the closure of Rust's
/// `Option::map`, both of `map_or_else`'s, or what Go's `sync.Once.Do` and C++'s
/// `std::call_once` are handed. The arguments are counted after the receiver, or after the
/// callee of a call in function form. Interface members are excluded, as for an extent role.
pub(super) fn run_once(facts: &Facts<'_, '_>, call: &Node) -> Option<&'static [u32]> {
    let reference = facts
        .reference(call)
        .filter(|_| call.kind == NodeKind::Call)?;
    let method = facts.declared(reference)?;
    let inherent = method.standard && !facts.interface_member(method);
    let key = inherent.then_some(method.operation.as_deref()).flatten()?;
    let handed = Contracts::embedded().handed(key)?;
    (handed.runs == Runs::AtMostOnce).then_some(handed.callbacks)
}

pub(super) fn place<'a>(
    facts: &Facts<'_, 'a>,
    expression: &'a Node,
    numbers: &std::collections::BTreeMap<&'a str, u32>,
) -> Result<Option<Place<'a>>, String> {
    place_at(facts, expression, numbers, 256)
}

fn place_at<'a>(
    facts: &Facts<'_, 'a>,
    expression: &'a Node,
    numbers: &std::collections::BTreeMap<&'a str, u32>,
    remaining: usize,
) -> Result<Option<Place<'a>>, String> {
    if remaining == 0 || !standard_adjustments(facts, expression) {
        return Ok(None);
    }
    let label = |root: &str| {
        let number = numbers.get(root).copied().unwrap_or(u32::MAX);
        format!("len({})#{number}", compact(&facts.snippet(expression)))
    };
    if expression.kind == NodeKind::Identifier {
        let Some(root) = facts.local(expression) else {
            return Ok(None);
        };
        let shared = facts.ty(expression).and_then(|ty| shared_target(facts, ty));
        return Ok(shared.map(|_| Place {
            root,
            fields: Vec::new(),
            path: Vec::new(),
            label: label(root),
        }));
    }

    let deref = expression.operator.as_deref() == Some("deref");
    let through = match expression.kind {
        NodeKind::FieldAccess => true,
        NodeKind::Operator if deref => true,
        NodeKind::Reference => expression.modifiers.is_empty(),
        _ => false,
    };
    if !through {
        return Ok(None);
    }
    let base = facts.body.child(expression, 0)?;
    if expression.kind == NodeKind::Operator && !is_shared_reference(facts.ty(base)) {
        return Ok(None);
    }
    match place_at(facts, base, numbers, remaining - 1)? {
        Some(place) if expression.kind == NodeKind::FieldAccess => {
            field_place(facts, expression, place, &label)
        }
        place => Ok(place),
    }
}

/// A field of a place rooted at a shared reference.
fn field_place<'a>(
    facts: &Facts<'_, 'a>,
    expression: &Node,
    mut place: Place<'a>,
    label: &dyn Fn(&str) -> String,
) -> Result<Option<Place<'a>>, String> {
    let root_type = facts
        .body
        .symbol(place.root)?
        .ty
        .as_deref()
        .and_then(|ty| facts.type_of(ty));
    if !is_shared_reference(root_type) {
        return Ok(None);
    }
    let index = expression.member_index.unwrap_or_default();
    place.fields.push(index as usize);
    place.path.push(Field {
        index,
        name: expression.member.clone().unwrap_or_default(),
    });
    place.label = label(place.root);

    Ok(Some(place))
}

// Labels are identities; source line breaks and indentation are not part of them.
fn compact(snippet: &str) -> String {
    snippet.split_whitespace().collect()
}

/// Adjustments that keep a place shared: built-in dereferences of shared references,
/// `Vec`'s shared dereference to its slice, shared borrows, and a shared array-to-slice
/// coercion that preserves the element type.
pub(super) fn standard_adjustments(facts: &Facts<'_, '_>, expression: &Node) -> bool {
    let Some(adjustments) = &expression.adjustments else {
        return true;
    };
    let mut current = facts.ty(expression);
    for adjustment in adjustments {
        let target = adjustment
            .target
            .as_deref()
            .and_then(|id| facts.type_of(id));
        let allowed = match (adjustment.kind.as_str(), adjustment.mode.as_deref()) {
            ("auto_deref", _) => is_shared_reference(current),
            ("overloaded_deref", Some("shared")) => {
                current.is_some_and(|ty| is_vec(facts, ty))
                    && target
                        .is_some_and(|ty| ty.kind == TypeKind::Sequence && ty.fixed != Some(true))
            }
            ("auto_ref", Some("shared")) => true,
            ("unsize", _) => shared_array_to_slice(facts, current, target),
            _ => false,
        };
        if !allowed {
            return false;
        }
        current = target;
    }
    true
}

fn shared_array_to_slice(facts: &Facts<'_, '_>, from: Option<&Type>, to: Option<&Type>) -> bool {
    let (Some(from), Some(to)) = (from, to) else {
        return false;
    };
    if !is_shared_reference(Some(from)) || !is_shared_reference(Some(to)) {
        return false;
    }
    let (Some(from), Some(to)) = (shared_target(facts, from), shared_target(facts, to)) else {
        return false;
    };

    from.kind == TypeKind::Sequence
        && from.fixed == Some(true)
        && to.kind == TypeKind::Sequence
        && to.fixed != Some(true)
        && from.element.is_some()
        && from.element == to.element
}

fn fixed_length(facts: &Facts<'_, '_>, ty: &Type) -> Option<u64> {
    let ty = shared_target(facts, ty)?;
    (ty.kind == TypeKind::Sequence && ty.fixed == Some(true))
        .then_some(ty.length)
        .flatten()
}

pub(super) fn iteration_receiver<'a>(
    facts: &Facts<'_, 'a>,
    head: &'a Node,
) -> Result<Option<&'a Node>, String> {
    if is_method(facts, head) && head.children.len() == 1 {
        return has_role(facts, head, ExtentRole::Traversal)
            .then(|| facts.body.child(head, 0))
            .transpose();
    }
    Ok(facts
        .ty(head)
        .is_some_and(|ty| extent_type(facts, ty))
        .then_some(head))
}

pub(super) fn extent_type(facts: &Facts<'_, '_>, ty: &Type) -> bool {
    shared_target(facts, ty).is_some_and(|ty| {
        (ty.kind == TypeKind::Sequence && ty.fixed != Some(true))
            || ty.kind == TypeKind::Set
            || is_vec(facts, ty)
    }) || (is_shared_reference(Some(ty)) && fixed_length(facts, ty).is_some())
}
