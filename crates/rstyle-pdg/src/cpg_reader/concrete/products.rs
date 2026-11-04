//! Local two-field structs instantiated over a lifetime and one scalar, observed as
//! scalar product instances where the profile follows slice-prefix products.

use super::{Capture, Failure, unsupported};
use crate::concrete_types::TypeTree;
use crate::scalar_product_instances::{Argument, Observation};
use rstyle_core::cpg::model::{Scope, Type};

impl<'a> Capture<'_, '_, 'a> {
    pub(super) fn scalar_product_instance(
        &mut self,
        ty: &'a Type,
        depth: usize,
    ) -> Result<Option<TypeTree>, Failure> {
        if !self.supports("rust.slice-prefix-product-results") {
            return Ok(None);
        }
        let Some(argument) = self.product_argument(ty)? else {
            return Ok(None);
        };
        let definition = self.definition_of(ty)?;
        let fields = ty.fields.as_deref().unwrap_or_default();
        let structure = definition
            .extensions
            .get("rust:compiler-identity")
            .and_then(|identity| identity.get("def_kind")?.as_str())
            == Some("Struct");
        if !structure
            || definition.scope != Scope::Unit
            || definition.destructor
            || self.visiting.contains(&definition.id.as_str())
            || fields.len() != 2
        {
            return Err(unsupported(
                "Scalar product instance lacks a bounded local declaration",
            ));
        }
        let source = self.declaration(definition)?;
        let declarations =
            crate::concrete_types::field_declarations(&source).map_err(Failure::Unsupported)?;
        if declarations.len() != 2 {
            return Err(unsupported(
                "Scalar product instance field roster disagrees",
            ));
        }
        self.visiting.push(definition.id.as_str());
        let captured = self.fields(fields, declarations, depth);
        self.visiting.pop();
        let observation = Observation {
            nominal: self.nominal(&definition.name, &source)?,
            source,
            argument,
            fields: captured?,
        };
        observation.check_shape().map_err(Failure::Unsupported)?;
        Ok(Some(TypeTree::ScalarProductInstance(Box::new(observation))))
    }

    /// The scalar a struct generic over one lifetime, then one type, is instantiated at.
    fn product_argument(&self, ty: &'a Type) -> Result<Option<Argument>, Failure> {
        let Some(definition) = ty
            .definition
            .as_deref()
            .and_then(|id| self.context.reader.tables.definition(id).ok())
        else {
            return Ok(None);
        };
        let own = definition
            .extensions
            .get("rust:compiler-identity")
            .and_then(|identity| identity.get("generics")?.get("own")?.as_array().cloned())
            .unwrap_or_default();
        let [argument] = ty.arguments.as_deref().unwrap_or_default() else {
            return Ok(None);
        };
        if own != ["lifetime", "type"] {
            return Ok(None);
        }
        let value = self.ty(argument)?;
        match value.spelling.as_str() {
            "u8" if value.kind == rstyle_core::cpg::model::TypeKind::Integer => {
                return Ok(Some(Argument::U8));
            }
            "u16" if value.kind == rstyle_core::cpg::model::TypeKind::Integer => {
                return Ok(Some(Argument::U16));
            }
            _ => {}
        }
        Ok(match self.nonzero(value)? {
            Some(TypeTree::NonZero(observation)) => Some(match observation.scalar {
                crate::nonzero_values::Scalar::U8 => Argument::NonZeroU8,
                crate::nonzero_values::Scalar::U16 => Argument::NonZeroU16,
            }),
            _ => None,
        })
    }
}
