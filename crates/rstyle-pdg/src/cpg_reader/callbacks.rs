//! Option callbacks: the named functions and closures a `core` `Option::map_or` or
//! `Iterator::position` call receives as its last argument.

use super::Context;
use crate::frontend::{Dispatch, HirNode, PathRes};
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

const RESULT_CALLBACKS: [&str; 6] = [
    "core::result::Result::<T, E>::and_then",
    "std::result::Result::<T, E>::and_then",
    "core::result::Result::<T, E>::map",
    "std::result::Result::<T, E>::map",
    "core::result::Result::<T, E>::map_err",
    "std::result::Result::<T, E>::map_err",
];
pub(super) const USIZE_READERS: &str = "rust.usize-reader-callback-observations";

type Children = Result<Vec<HirNode>, String>;

impl<'a> Context<'_, 'a> {
    /// A closure's parameters and body, where the profile observes the callback.
    pub(super) fn callback_children(&self, closure: &'a Node) -> Children {
        if self.usize_reader_callback(closure)? {
            return self.closure_children(closure, &|pattern| self.local_pattern(pattern));
        }
        if self
            .reader
            .profile
            .supports(crate::id_error_maps::CAPABILITY)
            && self.immediate_core_callback(closure, &["core::result::Result::<T, E>::map_err"])
        {
            return self.closure_children(closure, &|pattern| self.pattern(pattern));
        }
        if self
            .reader
            .profile
            .supports("rust.immediate-option-callbacks")
            && self.immediate_option_callback(closure)
        {
            return self.closure_children(closure, &|pattern| self.callback_pattern(pattern));
        }
        Ok(Vec::new())
    }

    fn closure_children(
        &self,
        closure: &'a Node,
        pattern: &dyn Fn(&'a Node) -> Result<HirNode, String>,
    ) -> Children {
        if closure
            .modifiers
            .iter()
            .any(|modifier| modifier == "coroutine")
        {
            return Ok(Vec::new());
        }
        let parts = self.body.children(closure)?;
        let Some((body, parameters)) = parts.split_last() else {
            return Ok(Vec::new());
        };
        parameters
            .iter()
            .map(|parameter| pattern(parameter))
            .chain(std::iter::once(self.expression(body)))
            .collect()
    }

    /// A callback parameter, where a shared reference pattern copies its scalar.
    fn callback_pattern(&self, pattern: &'a Node) -> Result<HirNode, String> {
        let node = self.pattern(pattern)?;
        let text = self.snippet(self.written(pattern));
        let shared = text.strip_prefix('&').is_some_and(|inner| {
            !inner.trim_start().starts_with("mut ") && !inner.trim_start().starts_with("pin ")
        });
        if !self.reader.profile.supports("rust.byte-iterator-position")
            || pattern.original_kind != "pat_ref"
            || !shared
        {
            return Ok(node);
        }
        let inner = self
            .parts(pattern)?
            .into_iter()
            .map(|inner| self.pattern(inner))
            .collect::<Result<_, _>>()?;
        Ok(HirNode {
            kind: "pat_shared_copy".into(),
            children: inner,
            ..node
        })
    }

    /// Whether a node is the only argument of one of the `core` methods `paths`.
    pub(super) fn immediate_core_callback(&self, node: &Node, paths: &[&str]) -> bool {
        let Some((_, arguments, definition)) = self.calling(node) else {
            return false;
        };
        arguments == 1
            && definition.scope != Scope::Unit
            && crate_name(definition) == Some("core")
            && paths.contains(&definition.name.as_str())
    }

    /// A `Result` callback, without a return, of a function reading a `(usize, usize)`.
    pub(super) fn usize_reader_callback(&self, closure: &'a Node) -> Result<bool, String> {
        Ok(self.reader.profile.supports(USIZE_READERS)
            && self.usize_reader_result()
            && self.immediate_core_callback(closure, &RESULT_CALLBACKS)
            && self.return_free(closure)?)
    }

    pub(super) fn usize_reader_result(&self) -> bool {
        let result = &self.body.function.signature.result;
        let Ok(result) = self.reader.ty(result) else {
            return false;
        };
        let is_result = result
            .definition
            .as_deref()
            .and_then(|definition| self.reader.tables.definition(definition).ok())
            .and_then(|definition| {
                definition
                    .extensions
                    .get("rust:compiler-identity")?
                    .get("diagnostic")?
                    .as_str()
                    .map(|name| name == "Result")
            })
            .unwrap_or(false);
        let Some(value) = result
            .arguments
            .as_deref()
            .and_then(|arguments| arguments.first())
            .and_then(|value| self.reader.ty(value).ok())
        else {
            return false;
        };
        let usize = |ty: &str| self.reader.ty(ty).is_ok_and(|ty| ty.spelling == "usize");
        is_result
            && value.kind == TypeKind::Tuple
            && matches!(value.items.as_deref(), Some([first, second]) if usize(first) && usize(second))
    }

    /// Whether a closure's body, nested closures included, has no `return`, within the
    /// observer's expression budget.
    fn return_free(&self, closure: &'a Node) -> Result<bool, String> {
        let Some(body) = self.body.children(closure)?.last().copied() else {
            return Ok(false);
        };
        let mut remaining = crate::concrete_types::MAX_ENTRIES;
        let mut pending = vec![(body, 1usize)];
        while let Some((node, depth)) = pending.pop() {
            let expression = node.original_kind.starts_with("expr_");
            if expression {
                if remaining == 0
                    || depth > crate::concrete_types::MAX_DEPTH
                    || node.original_kind == "expr_ret"
                    || node.kind == rstyle_core::cpg::model::NodeKind::Propagate
                {
                    return Ok(false);
                }
                remaining -= 1;
            }
            let depth = depth + usize::from(expression);
            pending.extend(
                self.body
                    .children(node)?
                    .into_iter()
                    .map(|child| (child, depth)),
            );
        }
        Ok(true)
    }

    /// A declaration's or reader callback's pattern, where a tuple of plain scalar
    /// bindings is observed as a scalar tuple.
    pub(super) fn local_pattern(&self, pattern: &'a Node) -> Result<HirNode, String> {
        let node = self.pattern(pattern)?;
        if pattern.original_kind != "pat_tuple" {
            return Ok(node);
        }
        let supports = |capability: &str| self.reader.profile.supports(capability);
        let children = self.parts(pattern)?;
        let scalar = |child: &&Node| {
            child.original_kind == "pat_binding"
                && self.declared(child).is_some_and(|(_, mutable)| !mutable)
                && child
                    .ty
                    .as_deref()
                    .and_then(|ty| self.reader.ty(ty).ok())
                    .is_some_and(|ty| {
                        matches!(ty.kind, TypeKind::Bool | TypeKind::Integer | TypeKind::Char)
                    })
        };
        let permitted = (supports("rust.named-error-validator-flow") || supports(USIZE_READERS))
            && !super::variants::rest(self, pattern)
            && !children.is_empty()
            && children.len() <= crate::concrete_types::MAX_FIELDS
            && children.iter().all(scalar);
        if !permitted {
            return Ok(node);
        }
        Ok(HirNode {
            kind: "pat_scalar_tuple".into(),
            children: children
                .into_iter()
                .map(|child| self.pattern(child))
                .collect::<Result<_, _>>()?,
            ..node
        })
    }
}
