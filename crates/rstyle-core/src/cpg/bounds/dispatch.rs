//! Standard iteration members that dispatch to trait implementations (`Clone`, `Ord`,
//! `Sum`, `FromIterator`) are modeled only when every dispatched type is standard, since
//! workspace implementations would otherwise run uncounted inside the synthetic scan.
//! Which types each member dispatches on is contract data; this module resolves them.

use super::super::model::{Node, Type, TypeKind};
use super::Collector;
use crate::runtime_bounds::contracts::{Dispatch, Dispatched};

const DEPTH: usize = 32;

impl<'a> Collector<'_, '_, 'a> {
    /// Whether the call on `receiver` can only reach standard trait implementations.
    pub(super) fn standard_dispatch(
        &self,
        expression: &'a Node,
        _receiver: &'a Node,
        dispatch: &Dispatch,
    ) -> Result<bool, String> {
        let reference = self.facts.reference(expression);
        let arguments: Vec<&str> = reference
            .and_then(|reference| reference.instance_arguments.as_ref())
            .map(|arguments| arguments.iter().map(String::as_str).collect())
            .unwrap_or_default();
        let generic = |index: u32| {
            arguments
                .get(index as usize)
                .and_then(|id| self.facts.type_of(id))
        };
        if let Some(agnostic) = &dispatch.item_agnostic
            && generic(agnostic.generic).is_some_and(|target| {
                self.facts
                    .type_key(target)
                    .is_some_and(|key| agnostic.types.iter().any(|name| name == key))
            })
        {
            return Ok(true);
        }
        let item = reference
            .and_then(|reference| reference.item.as_deref())
            .and_then(|id| self.facts.type_of(id));

        Ok(dispatch.standard.iter().all(|dispatched| {
            let ty = match dispatched {
                Dispatched::Item => item,
                Dispatched::Generic(index) => generic(*index),
            };
            ty.is_some_and(|ty| self.standard(ty, DEPTH))
        }))
    }

    // Closures and function items contribute only their outputs, which flow to the consumer.
    fn standard(&self, ty: &Type, depth: usize) -> bool {
        let Some(depth) = depth.checked_sub(1) else {
            return false;
        };
        let part = |id: &Option<String>| {
            id.as_deref()
                .and_then(|id| self.facts.type_of(id))
                .is_some_and(|inner| self.standard(inner, depth))
        };
        let all = |ids: &Option<Vec<String>>| {
            ids.iter().flatten().all(|id| {
                self.facts
                    .type_of(id)
                    .is_some_and(|inner| self.standard(inner, depth))
            })
        };
        match ty.kind {
            TypeKind::Bool
            | TypeKind::Char
            | TypeKind::Integer
            | TypeKind::Float
            | TypeKind::String
            | TypeKind::Bottom
            | TypeKind::Unit => true,
            TypeKind::Reference => part(&ty.target),
            TypeKind::Sequence => part(&ty.element),
            TypeKind::Tuple => all(&ty.items),
            TypeKind::Record | TypeKind::Sum => {
                self.facts.standard_definition(ty.definition.as_deref()) && all(&ty.arguments)
            }
            TypeKind::Pointer if ty.mode.as_deref() == Some("owned") => {
                self.facts.standard_definition(ty.definition.as_deref()) && all(&ty.arguments)
            }
            TypeKind::Closure => part(&ty.result),
            _ => false,
        }
    }
}
