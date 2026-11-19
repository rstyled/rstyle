//! Writes the bound program depends on. A direct value-only local that is neither
//! a modeled scalar nor an extent source needs no abstract state of its own.

use super::super::model::{DefinitionKind, Node, Type, TypeField, TypeKind, Variant};
use super::Collector;
use super::facts::Facts;

#[derive(Clone, Copy)]
struct Budget {
    work: u16,
    depth: u8,
}

impl Budget {
    fn enter(self) -> Option<Self> {
        Some(Self {
            work: self.work.checked_sub(1)?,
            depth: self.depth.checked_sub(1)?,
        })
    }
}

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
        let value = self
            .facts
            .ty(place)
            .is_some_and(|ty| Values(self.facts).stored(ty));
        // Symbols belonging to this frame are local storage or passed by value.
        // A reference identifier may write its referent, rather than rebind storage.
        let own = self.facts.body.symbol(symbol).is_ok_and(|symbol| {
            matches!(symbol.kind.as_str(), "local" | "parameter")
                && symbol.ty.as_deref() == place.ty.as_deref()
        });

        // The assignment expression is usually Unit, so inspect the old stored value too.
        if value && own && self.hidden_call(place).is_ok_and(|reason| reason.is_none()) {
            Write::Irrelevant
        } else {
            Write::Unmodeled
        }
    }
}

/// Certifies a closed value shape. Unknown members and exhausted work are failures,
/// including cycles in malformed or recursive type facts.
struct Values<'f, 'b, 'a>(&'f Facts<'b, 'a>);

impl Values<'_, '_, '_> {
    fn stored(&self, ty: &Type) -> bool {
        matches!(
            ty.kind,
            TypeKind::Bool
                | TypeKind::Char
                | TypeKind::Float
                | TypeKind::Unit
                | TypeKind::String
                | TypeKind::Tuple
                | TypeKind::Sequence
                | TypeKind::Record
                | TypeKind::Sum
        ) && self
            .remaining(
                ty,
                Budget {
                    work: 128,
                    depth: 32,
                },
            )
            .is_some()
    }

    fn remaining(&self, ty: &Type, budget: Budget) -> Option<u16> {
        let budget = budget.enter()?;
        if !self.nominal(ty) {
            return None;
        }
        let arguments = ty.arguments.as_deref().unwrap_or_default();
        let work = self.members(arguments.iter().map(String::as_str), budget)?;
        let budget = Budget { work, ..budget };
        match ty.kind {
            TypeKind::Integer
            | TypeKind::Bool
            | TypeKind::Char
            | TypeKind::Float
            | TypeKind::Unit
            | TypeKind::String => Some(budget.work),
            TypeKind::Tuple => self.members(ty.items.as_ref()?.iter().map(String::as_str), budget),
            TypeKind::Sequence => self.members(std::iter::once(ty.element.as_deref()?), budget),
            TypeKind::Record => self.fields(ty.fields.as_deref()?, budget),
            TypeKind::Sum => self.variants(ty.variants.as_deref()?, budget),
            _ => None,
        }
    }

    fn nominal(&self, ty: &Type) -> bool {
        match ty.definition.as_deref() {
            Some(id) => self.0.tables.definition(id).is_ok_and(|definition| {
                definition.kind == DefinitionKind::Type && !definition.destructor
            }),
            None => !matches!(ty.kind, TypeKind::Record | TypeKind::Sum),
        }
    }

    fn members<'t>(
        &self,
        mut members: impl Iterator<Item = &'t str>,
        budget: Budget,
    ) -> Option<u16> {
        members.try_fold(budget.work, |work, id| {
            self.remaining(self.0.type_of(id)?, Budget { work, ..budget })
        })
    }

    fn fields(&self, fields: &[TypeField], budget: Budget) -> Option<u16> {
        self.members(fields.iter().map(|field| field.ty.as_str()), budget)
    }

    fn variants(&self, variants: &[Variant], budget: Budget) -> Option<u16> {
        variants.iter().try_fold(budget.work, |work, variant| {
            let budget = Budget { work, ..budget }.enter()?;
            match variant.fields.as_deref() {
                Some(fields) => self.fields(fields, budget),
                None if variant.shape.as_deref() == Some("unit") => Some(budget.work),
                None => None,
            }
        })
    }
}
