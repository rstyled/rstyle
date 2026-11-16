//! Callback freedom of a selected parameterless method on a closed nominal state.
//! This proves no source length, remaining cardinality, purity or operation cost.

use super::Invocation;
use crate::cpg::model::{
    Callable, Definition, DefinitionKind, Dispatch, Ref, Scope, TargetKind, Type, TypeKind,
};
use crate::runtime_bounds::contracts::{StateArgument, StateCallbackFree};

struct Evidence<'a> {
    operation: &'a str,
    actual: &'a Type,
    owner: &'a Type,
}

impl<'a> Invocation<'_, 'a> {
    pub(super) fn state_certified(&self) -> bool {
        self.state_evidence().is_some_and(|evidence| {
            self.contracts
                .state_callback_free(evidence.operation)
                .iter()
                .any(|role| self.state_matches(&evidence, role))
        })
    }

    fn state_evidence(&self) -> Option<Evidence<'a>> {
        let reference = self.body.reference(&self.call.id)?;
        if !self.unenumerated(reference) || reference.dispatch != Some(Dispatch::Method) {
            return None;
        }
        let declared = self.state_declared(reference)?;
        let selected = self.state_selected(reference, declared)?;
        let owner = self.state_owner(selected)?;
        let [operand] = self.call.children.as_slice() else {
            return None;
        };
        let operand = self.body.node(operand).ok()?;
        let actual = self.borrowed(self.adjusted(operand)?, "exclusive")?;
        self.tables.ty(self.call.ty.as_deref()?).ok()?;
        if reference.signature.is_none()
            || !self.instantiated(&[operand])
            || reference.instance_arguments.as_deref() != Some(std::slice::from_ref(&actual.id))
        {
            return None;
        }
        Some(Evidence {
            operation: declared.operation.as_deref()?,
            actual,
            owner,
        })
    }

    fn state_declared(&self, reference: &Ref) -> Option<&'a Definition> {
        let target = reference.target.as_ref()?;
        if target.kind != TargetKind::Definition {
            return None;
        }
        let declared = self.tables.definition(target.id.as_deref()?).ok()?;
        fixed_member(declared)?;
        if declared.implements.is_some() || !declared.iteration {
            return None;
        }
        let owner = self.tables.definition(declared.owner.as_deref()?).ok()?;
        if owner.kind != DefinitionKind::Trait || !library(owner) {
            return None;
        }
        Some(declared)
    }

    fn state_selected(&self, reference: &Ref, declared: &Definition) -> Option<&'a Definition> {
        let target = reference.selected.as_ref()?;
        if target.kind != TargetKind::Definition {
            return None;
        }
        let selected = self.tables.definition(target.id.as_deref()?).ok()?;
        fixed_member(selected)?;
        if selected.implements.as_deref() != Some(declared.id.as_str())
            || ![declared.id.as_str(), selected.id.as_str()].contains(&self.callee.id.as_str())
            || !library(self.callee)
        {
            return None;
        }
        Some(selected)
    }

    fn state_owner(&self, selected: &Definition) -> Option<&'a Type> {
        let callable = selected.callable.as_ref()?;
        let owner = self.tables.ty(callable.owner_type.as_deref()?).ok()?;
        let receiver = self.tables.ty(callable.receiver.as_deref()?).ok()?;
        (self.borrowed(receiver, "exclusive")?.id == owner.id).then_some(owner)
    }

    fn borrowed(&self, ty: &Type, mode: &str) -> Option<&'a Type> {
        if ty.kind != TypeKind::Reference || ty.mode.as_deref() != Some(mode) {
            return None;
        }
        self.tables.ty(ty.target.as_deref()?).ok()
    }

    fn state_matches(&self, evidence: &Evidence<'_>, role: &StateCallbackFree) -> bool {
        let Evidence { actual, owner, .. } = evidence;
        if actual.kind != TypeKind::Record
            || owner.kind != TypeKind::Record
            || actual.definition.is_none()
            || actual.definition != owner.definition
        {
            return false;
        }
        let nominal = actual
            .definition
            .as_deref()
            .and_then(|id| self.tables.definition(id).ok());
        if !nominal.is_some_and(|definition| {
            library(definition)
                && definition.kind == DefinitionKind::Type
                && !definition.destructor
                && definition.stable_identity == role.receiver
        }) {
            return false;
        }
        self.state_arguments(actual, owner, role)
    }

    fn state_arguments(&self, actual: &Type, owner: &Type, role: &StateCallbackFree) -> bool {
        let (Some(actual), Some(owner)) = (&actual.arguments, &owner.arguments) else {
            return false;
        };
        actual.len() == role.arguments.len()
            && owner.len() == actual.len()
            && actual.iter().zip(owner).zip(&role.arguments).all(
                |((actual, owner), alternatives)| {
                    let actual = self.tables.ty(actual).ok();
                    let owner = self.tables.ty(owner).ok();
                    match (actual, owner) {
                        (Some(actual), Some(owner))
                            if owner.kind == TypeKind::Parameter || owner.id == actual.id =>
                        {
                            alternatives
                                .iter()
                                .any(|guard| self.state_argument(actual, guard))
                        }
                        _ => false,
                    }
                },
            )
    }

    fn state_argument(&self, actual: &Type, guard: &StateArgument) -> bool {
        let value = if guard.shared {
            self.borrowed(actual, "shared")
        } else {
            Some(actual)
        };
        value.is_some_and(|value| {
            value.key.as_deref() == Some(guard.key.as_str())
                && value.definition.is_none()
                && value.arguments.as_ref().is_none_or(Vec::is_empty)
                && matches!(
                    value.kind,
                    TypeKind::Integer
                        | TypeKind::Bool
                        | TypeKind::Char
                        | TypeKind::Float
                        | TypeKind::String
                        | TypeKind::Unit
                )
        })
    }
}

fn library(definition: &Definition) -> bool {
    definition.standard && definition.scope == Scope::Library
}

fn fixed_member(definition: &Definition) -> Option<&Callable> {
    let callable = definition.callable.as_ref()?;
    (library(definition)
        && definition.kind == DefinitionKind::Method
        && callable.receiver.is_some()
        && callable.generics == 0
        && callable.parameters.is_empty()
        && !definition
            .modifiers
            .iter()
            .chain(&callable.modifiers)
            .any(|modifier| modifier == "variadic"))
    .then_some(callable)
}

#[cfg(test)]
mod tests;
