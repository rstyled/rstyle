//! Call-site certificates for library bodies that invoke no caller-supplied code.
//! Evaluation of their receiver, arguments and surrounding cleanup is unchanged.

mod state;

use super::model::{
    Definition, DefinitionKind, Dispatch, Node, NodeKind, RefStatus, Scope, TargetKind, Type,
    TypeKind,
};
use super::tree::{Body, Tables};
use crate::runtime_bounds::contracts::{CallForm, CallbackFree, Contracts};

pub(super) fn callback_free(
    body: &Body<'_>,
    tables: &Tables<'_>,
    call: &Node,
    callee: &Definition,
) -> bool {
    let invocation = Invocation {
        body,
        tables,
        call,
        callee,
        contracts: Contracts::embedded(),
    };
    invocation.certified() || invocation.state_certified()
}

struct Invocation<'b, 'a> {
    body: &'b Body<'a>,
    tables: &'b Tables<'a>,
    call: &'a Node,
    callee: &'a Definition,
    contracts: &'b Contracts,
}

impl<'a> Invocation<'_, 'a> {
    fn certified(&self) -> bool {
        let Some(role) = self
            .callee
            .operation
            .as_deref()
            .and_then(|key| self.contracts.callback_free(key))
        else {
            return false;
        };
        if !self.resolved() || !self.layout(role) {
            return false;
        }
        let Ok(operands) = self.operands(role) else {
            return false;
        };
        if !self.instantiated(&operands) {
            return false;
        }
        role.types.iter().all(|guard| {
            self.adjusted(operands[guard.argument as usize])
                .is_some_and(|ty| self.keyed(ty, &guard.keys))
        }) && role.cleanup_free.iter().all(|&index| {
            let holds = role
                .types
                .iter()
                .any(|guard| guard.argument == index && guard.holds_arguments);
            self.adjusted(operands[index as usize])
                .is_some_and(|ty| Cleanup(self.tables).operand(ty, holds).is_some())
        })
    }

    /// A coarse may-call flag can be refined; enumerated execution is never discarded.
    fn resolved(&self) -> bool {
        let Some(reference) = self.body.reference(&self.call.id) else {
            return false;
        };
        self.unenumerated(reference)
            && reference.target.as_ref().is_some_and(|target| {
                target.kind == TargetKind::Definition
                    && target.id.as_deref() == Some(self.callee.id.as_str())
            })
            && reference.selected.as_ref().is_none_or(|target| {
                target.kind == TargetKind::Definition
                    && target.id.as_deref() == Some(self.callee.id.as_str())
            })
            && self.callee.standard
            && self.callee.scope == Scope::Library
            && self.callee.implements.is_none()
            && self
                .callee
                .owner
                .as_deref()
                .and_then(|id| self.tables.definition(id).ok())
                .is_none_or(|owner| owner.kind != DefinitionKind::Trait)
    }

    fn unenumerated(&self, reference: &super::model::Ref) -> bool {
        self.call.kind == NodeKind::Call
            && reference.status == RefStatus::Resolved
            && reference.arguments.is_none()
            && reference
                .calls_back
                .as_ref()
                .is_none_or(|callbacks| callbacks.runs.is_none())
            && reference.drives.is_none()
    }

    fn layout(&self, role: &CallbackFree) -> bool {
        let Some(callable) = &self.callee.callable else {
            return false;
        };
        if callable.parameters.len() != role.arity as usize
            || self
                .callee
                .modifiers
                .iter()
                .chain(&callable.modifiers)
                .any(|modifier| modifier == "variadic")
        {
            return false;
        }
        let dispatch = self
            .body
            .reference(&self.call.id)
            .and_then(|reference| reference.dispatch);
        match role.form {
            CallForm::Member => {
                self.callee.kind == DefinitionKind::Method
                    && callable.owner_type.is_some()
                    && callable.receiver.is_some()
                    && dispatch == Some(Dispatch::Method)
            }
            CallForm::Function => {
                self.callee.kind == DefinitionKind::Function
                    && callable.receiver.is_none()
                    && matches!(dispatch, Some(Dispatch::Static | Dispatch::Builtin))
            }
        }
    }

    fn operands(&self, role: &CallbackFree) -> Result<Vec<&'a Node>, String> {
        let member = role.form == CallForm::Member;
        let count = role.arity as usize + 1;
        if self.call.children.len() != count {
            return Err("Invocation arity mismatch".into());
        }
        let children = self.body.children(self.call)?;
        if member {
            return Ok(children);
        }
        let callee = children[0];
        let named = self
            .body
            .reference(&callee.id)
            .and_then(|reference| reference.target.as_ref());
        if callee.kind != NodeKind::Identifier
            || !callee.children.is_empty()
            || callee
                .adjustments
                .as_ref()
                .is_some_and(|adjustments| !adjustments.is_empty())
            || named.is_none_or(|target| {
                target.kind != TargetKind::Definition
                    || target.id.as_deref() != Some(self.callee.id.as_str())
            })
        {
            return Err("Invocation does not name its function".into());
        }
        Ok(children[1..].to_vec())
    }

    fn instantiated(&self, operands: &[&Node]) -> bool {
        let signature = self
            .body
            .reference(&self.call.id)
            .and_then(|reference| reference.signature.as_ref());
        let Some(signature) = signature else {
            return true;
        };
        signature.parameters.len() == operands.len()
            && signature
                .parameters
                .iter()
                .zip(operands)
                .all(|(expected, operand)| {
                    self.adjusted(operand)
                        .is_some_and(|ty| expected.as_deref() == Some(ty.id.as_str()))
                })
            && signature
                .result
                .as_deref()
                .is_some_and(|result| self.call.ty.as_deref() == Some(result))
    }

    fn adjusted(&self, node: &Node) -> Option<&'a Type> {
        let id = match node
            .adjustments
            .as_deref()
            .and_then(|adjustments| adjustments.last())
        {
            Some(adjustment) => adjustment.target.as_deref()?,
            None => node.ty.as_deref()?,
        };
        self.tables.ty(id).ok()
    }

    fn keyed(&self, mut ty: &'a Type, keys: &[String]) -> bool {
        for _ in 0..32 {
            if ty.kind != TypeKind::Reference {
                return ty.key.as_ref().is_some_and(|key| keys.contains(key));
            }
            if !matches!(ty.mode.as_deref(), Some("shared" | "exclusive")) {
                return false;
            }
            let Some(target) = ty.target.as_deref().and_then(|id| self.tables.ty(id).ok()) else {
                return false;
            };
            ty = target;
        }
        false
    }
}

#[derive(Clone, Copy)]
struct Budget {
    depth: u8,
    work: u16,
}

struct Cleanup<'b, 'a>(&'b Tables<'a>);

impl Cleanup<'_, '_> {
    /// What's left of the budget once an operand's cleanup runs no workspace code. A value its
    /// contract says holds exactly its type arguments is read through them where the frontend
    /// spells out neither its variants nor its fields, as for a library enum.
    fn operand(&self, ty: &Type, holds_arguments: bool) -> Option<u16> {
        let budget = Budget {
            depth: 32,
            work: 128,
        };
        if holds_arguments && ty.variants.is_none() && ty.fields.is_none() {
            if !self.0.no_own_destructor(ty) {
                return None;
            }
            let arguments = ty.arguments.as_deref()?.iter().map(String::as_str);
            return self.members(arguments, budget);
        }
        self.remaining(ty, budget)
    }

    fn remaining(&self, ty: &Type, budget: Budget) -> Option<u16> {
        let budget = Budget {
            depth: budget.depth.checked_sub(1)?,
            work: budget.work.checked_sub(1)?,
        };
        if ty.kind == TypeKind::Reference {
            return (matches!(ty.mode.as_deref(), Some("shared" | "exclusive"))
                && ty.target.as_deref().is_some_and(|id| self.0.ty(id).is_ok()))
            .then_some(budget.work);
        }
        if !self.0.no_own_destructor(ty) {
            return None;
        }
        match ty.kind {
            TypeKind::Integer
            | TypeKind::Bool
            | TypeKind::Char
            | TypeKind::Float
            | TypeKind::String
            | TypeKind::Unit
            | TypeKind::Function => Some(budget.work),
            TypeKind::Tuple => {
                self.members(ty.items.as_deref()?.iter().map(String::as_str), budget)
            }
            TypeKind::Sequence => self.members(std::iter::once(ty.element.as_deref()?), budget),
            TypeKind::Record => self.members(
                ty.fields.as_deref()?.iter().map(|field| field.ty.as_str()),
                budget,
            ),
            TypeKind::Sum => {
                ty.variants
                    .as_deref()?
                    .iter()
                    .try_fold(budget.work, |work, variant| {
                        let budget = Budget {
                            work: work.checked_sub(1)?,
                            ..budget
                        };
                        match variant.fields.as_deref() {
                            Some(fields) => {
                                self.members(fields.iter().map(|field| field.ty.as_str()), budget)
                            }
                            None if variant.shape.as_deref() == Some("unit") => Some(budget.work),
                            None => None,
                        }
                    })
            }
            _ => None,
        }
    }

    fn members<'t>(&self, mut ids: impl Iterator<Item = &'t str>, budget: Budget) -> Option<u16> {
        ids.try_fold(budget.work, |work, id| {
            self.remaining(self.0.ty(id).ok()?, Budget { work, ..budget })
        })
    }
}

#[cfg(test)]
mod tests;
