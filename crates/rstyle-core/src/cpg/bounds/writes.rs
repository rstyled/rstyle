//! Writes the bound program depends on. A direct primitive local that is neither
//! a modeled scalar nor an extent source needs no abstract state of its own.

use super::super::model::{Node, TypeKind};
use super::Collector;

pub(super) enum Write {
    Scalar(u32),
    Irrelevant,
    Unmodeled,
}

impl Collector<'_, '_, '_> {
    pub(super) fn written(&self, place: &Node) -> Write {
        if let Some(variable) = self.local_id(place) {
            return Write::Scalar(variable);
        }
        let Some(symbol) = self.facts.local(place) else {
            return Write::Unmodeled;
        };
        if self.is_extent_source(place)
            || self.facts.adjusted_of(place)
            || !place.children.is_empty()
        {
            return Write::Unmodeled;
        }
        let primitive = self.facts.ty(place).is_some_and(|ty| {
            matches!(
                ty.kind,
                TypeKind::Bool | TypeKind::Char | TypeKind::Float | TypeKind::Unit
            )
        });
        // Symbols belonging to this frame are local storage or passed by value.
        // References, fields, indexes, aggregates and missing type facts stay unknown.
        let own = self.facts.body.symbol(symbol).is_ok_and(|symbol| {
            matches!(symbol.kind.as_str(), "local" | "parameter")
                && symbol.ty.as_deref() == place.ty.as_deref()
        });

        if primitive && own {
            Write::Irrelevant
        } else {
            Write::Unmodeled
        }
    }
}
