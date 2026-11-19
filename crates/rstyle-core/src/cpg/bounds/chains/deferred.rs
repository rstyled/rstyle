//! A single adjacent local can retain a pipeline without moving its initialization.

use super::{Callback, Chain, Collector, Consumer, MAX_STAGES, Stage};
use crate::cpg::bounds::Local;
use crate::cpg::bounds::deferred::binding;
use crate::cpg::bounds::extent::{has_role, standard_adjustments};
use crate::cpg::model::{
    Definition, DefinitionKind, Dispatch, Node, NodeKind, Ref, RefStatus, Scope, TargetKind,
    TypeKind,
};
use crate::runtime_bounds::contracts::{Contracts, ExtentRole};

mod runs;

impl<'a> Collector<'_, '_, 'a> {
    pub(in crate::cpg::bounds) fn prepare_deferred(
        &mut self,
        root: &'a Node,
    ) -> Result<(), String> {
        if root.kind != NodeKind::Block || root.label.is_some() || !root.modifiers.is_empty() {
            return Ok(());
        }
        let parts = self.facts.body.children(root)?;
        let [local, tail] = parts.as_slice() else {
            return Ok(());
        };
        let Some(binding) = binding(self.facts, local, tail)? else {
            return Ok(());
        };
        let Some(chain) = self.chain(binding.init)? else {
            return Ok(());
        };
        if !self.deferred_initializer(&chain, binding.init)? {
            return Ok(());
        }
        let use_node = binding.use_node;
        self.deferred = Some(binding);
        let receiver = self.facts.body.child(tail, 0)?;
        if !self.deferred_continuation(tail, use_node)?
            || !self
                .chain(receiver)?
                .is_some_and(|chain| chain.standard && self.deferred_environments(&chain))
        {
            self.deferred = None;
        }
        Ok(())
    }

    pub(in crate::cpg::bounds) fn initialize_deferred(
        &mut self,
        parts: &Local<'a>,
        next: u32,
    ) -> Result<Option<u32>, String> {
        let Some(binding) = &self.deferred else {
            return Ok(None);
        };
        if self.binding_symbol(parts.pattern) != Some(binding.symbol) {
            return Ok(None);
        }
        let Some(chain) = self.chain(binding.init)? else {
            return Ok(None);
        };
        self.chain_inputs(&chain, next).map(Some)
    }

    fn deferred_initializer(&self, chain: &Chain<'a>, init: &'a Node) -> Result<bool, String> {
        if !chain.standard || chain.stages.is_empty() || chain.initialized {
            return Ok(false);
        }
        if !self.deferred_source(chain.root)? {
            return Ok(false);
        }
        let mut current = init;
        let mut work = 256;
        for stage in chain.stages.iter().rev() {
            let Some(role) = self
                .iterator_method(current)
                .and_then(|key| Contracts::embedded().stage(key))
            else {
                return Ok(false);
            };
            if !role.deferred
                || !self.deferred_call(current)
                || !self.deferred_state(current)
                || !self.deferred_item(current, &mut work)
            {
                return Ok(false);
            }
            let callback = match stage {
                Stage::Map(callback) | Stage::Filter(callback) | Stage::TakeWhile(callback) => {
                    Some(*callback)
                }
                Stage::FlatMap(_) => return Ok(false),
                _ => None,
            };
            if callback.is_some_and(|callback| !self.deferred_capture(callback)) {
                return Ok(false);
            }
            current = self.facts.body.child(current, 0)?;
        }
        Ok(current.id == chain.root.id
            && chain
                .arguments
                .iter()
                .all(|argument| self.for_item_type(self.facts.adjusted(argument), 32, &mut work)))
    }

    fn deferred_source(&self, root: &'a Node) -> Result<bool, String> {
        if !has_role(self.facts, root, ExtentRole::Traversal)
            || root.children.len() != 1
            || !self.deferred_call(root)
        {
            return Ok(false);
        }
        let receiver = self.facts.body.child(root, 0)?;
        if !standard_adjustments(self.facts, receiver) {
            return Ok(false);
        }
        Ok(self
            .place(receiver)?
            .is_some_and(|place| self.formals.contains_key(place.root)))
    }

    // Only the initializer's yielded value crosses the local's lifetime boundary.
    fn deferred_continuation(&self, tail: &'a Node, use_node: &Node) -> Result<bool, String> {
        let children = self.facts.body.children(tail)?;
        let Some((&receiver, arguments)) = children.split_first() else {
            return Ok(false);
        };
        let Some((consumer, role)) = self.consumer(tail, arguments)? else {
            return Ok(false);
        };
        if !self.deferred_call(tail) || !self.standard_dispatch(tail, receiver, &role.dispatch)? {
            return Ok(false);
        }
        let mut current = tail;
        let mut work = 256;
        if let Consumer::Each(callback) | Consumer::Search(callback) = consumer
            && !self.for_callback_cleanup(callback, &mut work)
        {
            return Ok(false);
        }
        for _ in 0..=MAX_STAGES {
            if !self.deferred_call(current) {
                return Ok(false);
            }
            let inner = self.facts.body.child(current, 0)?;
            if inner.id == use_node.id {
                return Ok(self.deferred_item(current, &mut work));
            }
            if self
                .iterator_method(inner)
                .and_then(|key| Contracts::embedded().stage(key))
                .is_none()
            {
                return Ok(false);
            }
            current = inner;
        }
        Ok(false)
    }

    fn deferred_item(&self, call: &Node, work: &mut usize) -> bool {
        let item = self
            .facts
            .reference(call)
            .and_then(|reference| reference.item.as_deref())
            .and_then(|id| self.facts.type_of(id));
        self.for_item_type(item, 32, work)
    }

    fn deferred_environments(&self, chain: &Chain<'a>) -> bool {
        let mut work = 256;
        chain.stages.iter().all(|stage| match stage {
            Stage::Map(callback) | Stage::Filter(callback) | Stage::TakeWhile(callback) => {
                self.for_callback_cleanup(*callback, &mut work)
            }
            Stage::FlatMap(_) => false,
            _ => true,
        })
    }

    fn deferred_capture(&self, callback: Callback<'a>) -> bool {
        let Callback::Closure(closure) = callback else {
            return true;
        };
        if !closure.modifiers.is_empty() || self.facts.adjusted_of(closure) {
            return false;
        }
        closure.captures.as_ref().is_some_and(|captures| {
            captures.iter().all(|capture| {
                let Ok(symbol) = self.facts.body.symbol(&capture.symbol) else {
                    return false;
                };
                let ty = self.facts.type_of(&capture.ty);
                capture.mode == "shared"
                    && capture.path.is_empty()
                    && self.formals.contains_key(capture.symbol.as_str())
                    && !symbol.mutable
                    && symbol.ty.as_deref() == Some(capture.ty.as_str())
                    && ty.is_some_and(|ty| {
                        matches!(
                            ty.kind,
                            TypeKind::Integer | TypeKind::Bool | TypeKind::Char | TypeKind::Float
                        )
                    })
            })
        })
    }

    /// The new state route requires complete resolved calls, not just method spellings.
    fn deferred_call(&self, call: &'a Node) -> bool {
        let Some(reference) = self.facts.reference(call) else {
            return false;
        };
        let Some(definition) = self.facts.declared(reference) else {
            return false;
        };
        if !self.deferred_resolution(reference, definition)
            || !self.deferred_layout(call, definition)
        {
            return false;
        }
        self.deferred_signature(call, reference) && self.deferred_runs(call, reference)
    }

    fn deferred_resolution(&self, reference: &Ref, definition: &Definition) -> bool {
        reference.status == RefStatus::Resolved
            && reference.dispatch == Some(Dispatch::Method)
            && reference.arguments.is_none()
            && reference.drives.is_none()
            && reference
                .target
                .as_ref()
                .is_some_and(|target| target.kind == TargetKind::Definition)
            && reference.selected.as_ref().is_none_or(|target| {
                target.kind == TargetKind::Definition
                    && target.id.as_deref() == Some(definition.id.as_str())
            })
            && (!definition.iteration || reference.selected.is_some())
            && definition.kind == DefinitionKind::Method
            && definition.standard
            && definition.scope == Scope::Library
    }

    fn deferred_layout(&self, call: &Node, definition: &Definition) -> bool {
        let Some(callable) = &definition.callable else {
            return false;
        };
        let Ok(receiver) = self.facts.body.child(call, 0) else {
            return false;
        };
        call.kind == NodeKind::Call
            && !self.facts.adjusted_of(call)
            && callable.receiver.is_some()
            && call.children.len() == callable.parameters.len() + 1
            && !definition
                .modifiers
                .iter()
                .chain(&callable.modifiers)
                .any(|modifier| modifier == "variadic")
            && (!definition.iteration || !self.facts.adjusted_of(receiver))
            && (definition.iteration || self.deferred_state(call))
    }

    fn deferred_signature(&self, call: &Node, reference: &Ref) -> bool {
        let Some(signature) = &reference.signature else {
            return false;
        };
        signature.parameters.len() == call.children.len()
            && signature.result.as_deref().is_some_and(|result| {
                call.ty.as_deref() == Some(result) && self.facts.type_of(result).is_some()
            })
            && signature
                .parameters
                .iter()
                .zip(&call.children)
                .all(|(expected, child)| {
                    let Ok(child) = self.facts.body.node(child) else {
                        return false;
                    };
                    let ty = match child
                        .adjustments
                        .as_deref()
                        .and_then(|adjustments| adjustments.last())
                    {
                        Some(adjustment) => adjustment.target.as_deref(),
                        None => child.ty.as_deref(),
                    };
                    expected.as_deref().is_some_and(|expected| {
                        ty == Some(expected) && self.facts.type_of(expected).is_some()
                    })
                })
    }

    fn deferred_state(&self, call: &Node) -> bool {
        let Some(ty) = self.facts.ty(call) else {
            return false;
        };
        ty.kind == TypeKind::Record
            && self.facts.tables.no_own_destructor(ty)
            && ty
                .definition
                .as_deref()
                .and_then(|id| self.facts.tables.definition(id).ok())
                .is_some_and(|definition| {
                    definition.kind == DefinitionKind::Type
                        && definition.standard
                        && definition.scope == Scope::Library
                })
    }
}

#[cfg(test)]
mod tests;
