//! Option callbacks: the named functions and closures a `core` `Option::map_or` or
//! `Iterator::position` call receives as its last argument.

use super::Context;
use crate::frontend::{Dispatch, PathRes};
use rstyle_core::cpg::model::{Definition, Node, Origin, Scope, TargetKind, TypeKind};

const MAP_OR: [&str; 2] = [
    "core::option::Option::<T>::map_or",
    "std::option::Option::<T>::map_or",
];
const POSITION: [&str; 2] = [
    "core::iter::Iterator::position",
    "std::iter::Iterator::position",
];

impl<'a> Context<'_, 'a> {
    /// The method call a node is the last argument of, with its argument count.
    fn calling(&self, node: &Node) -> Option<(&'a Node, usize, &'a Definition)> {
        let parent = *self.parents.get(node.id.as_str())?;
        if parent.original_kind != "expr_method_call" {
            return None;
        }
        let children = self.parts(parent).ok()?;
        if children.last()?.id != node.id {
            return None;
        }
        let reference = self.body.reference(&parent.id)?;
        let declared = match &reference.target {
            Some(target) => target.id.as_deref()?,
            None => reference.via.as_deref()?,
        };
        let definition = self.reader.tables.definition(declared).ok()?;
        Some((parent, children.len() - 1, definition))
    }

    /// Whether a node is the callback of a `core` `Option::map_or`, or of
    /// `Iterator::position` where the profile observes byte positions.
    pub(super) fn immediate_option_callback(&self, node: &Node) -> bool {
        let Some((_, arguments, definition)) = self.calling(node) else {
            return false;
        };
        let path = definition.name.as_str();
        let option = arguments == 2 && MAP_OR.contains(&path);
        let position = self.reader.profile.supports("rust.byte-iterator-position")
            && arguments == 1
            && POSITION.contains(&path);
        definition.scope != Scope::Unit
            && crate_name(definition) == Some("core")
            && (option || position)
    }

    /// A local `fn(u8) -> bool` named as a `map_or` callback, with its call dispatch.
    pub(super) fn named_option_function(&self, node: &Node) -> Option<(PathRes, Dispatch)> {
        if !self.reader.profile.supports("rust.named-option-callbacks")
            || node.original_kind != "expr_path"
            || node.provenance.origin != Origin::Source
            || node
                .adjustments
                .as_ref()
                .is_some_and(|adjustments| !adjustments.is_empty())
        {
            return None;
        }
        let target = self.body.reference(&node.id)?.target.as_ref()?;
        if target.kind != TargetKind::Definition {
            return None;
        }
        let definition = self.reader.tables.definition(target.id.as_deref()?).ok()?;
        let compiler = definition.extensions.get("rust:compiler-identity")?;
        let generics = compiler.get("generics")?.get("count")?.as_u64()?;
        if definition.scope != Scope::Unit
            || compiler.get("def_kind")?.as_str()? != "Fn"
            || generics != 0
            || !self.immediate_option_callback(node)
            || self.calling(node)?.1 != 2
        {
            return None;
        }
        let callable = definition.callable.as_ref()?;
        let [parameter] = callable.parameters.as_slice() else {
            return None;
        };
        let parameter = self.reader.ty(parameter).ok()?;
        let result = self.reader.ty(&callable.result).ok()?;
        if parameter.kind != TypeKind::Integer
            || parameter.spelling != "u8"
            || result.kind != TypeKind::Bool
            || callable.receiver.is_some()
        {
            return None;
        }
        let resolution = self.helper(definition, ("resolved_function_item", None), None)?;
        let dispatch = Dispatch {
            kind: "resolved_function_item".into(),
            op: definition.name.clone(),
            operand_tys: vec!["u8".into()],
            result_ty: "bool".into(),
        };
        Some((resolution, dispatch))
    }
}

fn crate_name(definition: &Definition) -> Option<&str> {
    definition
        .extensions
        .get("rust:compiler-identity")?
        .get("crate_name")?
        .as_str()
}
