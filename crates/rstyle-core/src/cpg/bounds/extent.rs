//! Immutable standard-library extents, identified by resolved types and by the extent
//! roles their methods have in the library contracts.

use super::super::model::{Node, NodeKind, Type, TypeKind};
use super::Collector;
use super::facts::{Facts, is_shared_reference, shared_target};
use crate::runtime_bounds::contracts::{Contracts, ExtentRole};
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
        Ok(self
            .place(receiver)?
            .and_then(|place| self.extents.get(&place.key()))
            .map(|label| Value::Input(label.clone())))
    }

    pub(super) fn place(&self, expression: &'a Node) -> Result<Option<Place<'a>>, String> {
        place(self.facts, expression, &self.numbers)
    }
}

pub(super) fn is_vec(facts: &Facts<'_, '_>, ty: &Type) -> bool {
    facts.type_key(ty) == Some("Vec") && facts.standard_definition(ty.definition.as_deref())
}

/// The method call's receiver, when the call reads a standard extent's length.
pub(super) fn extent_receiver<'a>(
    facts: &Facts<'_, 'a>,
    expression: &'a Node,
) -> Result<Option<&'a Node>, String> {
    if !is_method(facts, expression) || expression.children.len() != 1 {
        return Ok(None);
    }
    if !has_role(facts, expression, ExtentRole::Length) {
        return Ok(None);
    }
    facts.body.child(expression, 0).map(Some)
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

    let inherent = method.standard
        && method.implements.is_none()
        && !facts.interface_member(method)
        && method
            .callable
            .as_ref()
            .is_some_and(|c| c.owner_type.is_some());
    let key = inherent.then_some(method.operation.as_deref()).flatten();
    key.and_then(|key| Contracts::embedded().extent(key))
        .is_some_and(|extent| extent.role == role)
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
/// `Vec`'s shared dereference to its slice, and shared borrows.
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
            _ => false,
        };
        if !allowed {
            return false;
        }
        current = target;
    }
    true
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
        (ty.kind == TypeKind::Sequence && ty.fixed != Some(true)) || is_vec(facts, ty)
    })
}
