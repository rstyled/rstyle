//! Local enums in the concrete type grammar: fieldless, mixed and scalar-payload.

use super::{Capture, Failure, unsupported};
use crate::concrete_types::{Field, MixedVariant, TypeTree, Variant, VariantShape};
use rstyle_core::cpg::model::{Definition, Scope, Type, Variant as Declared};

impl<'a> Capture<'_, '_, 'a> {
    pub(super) fn enumeration(&mut self, ty: &'a Type, depth: usize) -> Result<TypeTree, Failure> {
        let variants = ty.variants.as_deref().unwrap_or_default();
        let has = |modifier: &str| ty.modifiers.iter().any(|m| m == modifier);
        // A library sum records its variant shapes as modifiers instead of its variants.
        let (fieldless, mixed) = match ty.variants {
            Some(_) => (
                variants.iter().all(|variant| fields(variant).is_empty()),
                variants
                    .iter()
                    .any(|variant| fields(variant).is_empty() || shape(variant) == "record"),
            ),
            None => (
                has("rust:fieldless-variants"),
                has("rust:unit-or-record-variant"),
            ),
        };
        if self.supports("rust.fieldless-enum-values") && fieldless {
            self.fieldless_enum(ty, variants)
        } else if self.supports("rust.borrowed-mixed-owners") && mixed {
            self.mixed_enum(ty, variants, depth)
        } else {
            self.scalar_enum(ty, variants, depth)
        }
    }

    /// A local, nongeneric, destructor-free enum's definition.
    fn local_enum(&self, ty: &Type, reason: &str) -> Result<&'a Definition, Failure> {
        let definition = self.definition_of(ty)?;
        let generic = ty
            .arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty())
            || ty.modifiers.iter().any(|m| m == "rust:lifetime-arguments");
        if definition.scope != Scope::Unit || generic || definition.destructor {
            return Err(unsupported(reason));
        }
        Ok(definition)
    }

    fn fieldless_enum(
        &mut self,
        ty: &'a Type,
        declared: &'a [Declared],
    ) -> Result<TypeTree, Failure> {
        let reason = "Fieldless enum requires local concrete destructor-free 64-bit context";
        let definition = self.local_enum(ty, reason)?;
        if self.context.reader.unit.target.pointer_width != 64 {
            return Err(unsupported(reason));
        }
        let source = self.declaration(definition)?;
        let variants = match crate::concrete_types::fieldless_variants(&source) {
            Ok(variants) => variants,
            Err(error) => self.explicit_u8(ty, &source, error)?,
        };
        if variants.len() != declared.len() {
            return Err(unsupported("Fieldless enum inventory disagrees"));
        }
        for (variant, (name, expected)) in declared.iter().zip(&variants) {
            if !fields(variant).is_empty()
                || variant.name != *name
                || variant.discriminant != Some(*expected)
            {
                return Err(unsupported(
                    "Resolved fieldless discriminant disagrees with source",
                ));
            }
        }
        Ok(TypeTree::FieldlessEnum {
            nominal: self.nominal(&definition.name, &source)?,
            source,
            variants,
        })
    }

    fn explicit_u8(
        &self,
        ty: &Type,
        source: &str,
        historical: String,
    ) -> Result<Vec<(String, i64)>, Failure> {
        if !self.supports(crate::concrete_types::EXPLICIT_U8_CAPABILITY) {
            return Err(Failure::Unsupported(historical));
        }
        if !ty
            .modifiers
            .iter()
            .any(|modifier| modifier == "rust:repr-u8")
        {
            return Err(unsupported(
                "Extended fieldless enum requires exact compiler repr(u8)",
            ));
        }
        crate::concrete_types::explicit_u8_fieldless_variants(source).map_err(Failure::Unsupported)
    }

    fn mixed_enum(
        &mut self,
        ty: &'a Type,
        declared: &'a [Declared],
        depth: usize,
    ) -> Result<TypeTree, Failure> {
        let reason = "Mixed enum requires finite local nongeneric destructor-free source";
        let definition = self.local_enum(ty, reason)?;
        if self.visiting.contains(&definition.id.as_str()) {
            return Err(unsupported(reason));
        }
        let source = self.declaration(definition)?;
        let declarations = crate::concrete_types::mixed_variant_declarations(&source)
            .map_err(Failure::Unsupported)?;
        if declarations.len() != declared.len() {
            return Err(unsupported(
                "Mixed enum source/resolved variant inventories disagree",
            ));
        }

        self.visiting.push(definition.id.as_str());
        let captured: Result<Vec<_>, _> = declared
            .iter()
            .zip(declarations)
            .enumerate()
            .map(|(position, (variant, (name, declaration)))| {
                if variant.name != name {
                    return Err(unsupported("Mixed enum variant identity/order disagrees"));
                }
                let shape = self.mixed_shape(variant, declaration, depth)?;
                Ok(MixedVariant {
                    position,
                    name,
                    shape,
                })
            })
            .collect();
        self.visiting.pop();
        let tree = TypeTree::MixedEnum {
            nominal: self.nominal(&definition.name, &source)?,
            source,
            variants: captured?,
        };
        crate::concrete_types::validate(&tree).map_err(Failure::Unsupported)?;
        Ok(tree)
    }

    fn mixed_shape(
        &mut self,
        variant: &'a Declared,
        declaration: VariantShape<String>,
        depth: usize,
    ) -> Result<VariantShape<Field>, Failure> {
        let declared = fields(variant);
        match declaration {
            VariantShape::Unit {} if declared.is_empty() && shape(variant) == "unit" => {
                Ok(VariantShape::Unit {})
            }
            VariantShape::SingleTuple { field: declaration }
                if declared.len() == 1 && shape(variant) == "tuple" =>
            {
                let ty = self.tree(&declared[0].ty, depth + 1)?;
                let field = Field {
                    position: 0,
                    name: "0".into(),
                    declaration: declaration.into(),
                    ty,
                };
                Ok(VariantShape::SingleTuple { field })
            }
            VariantShape::Named { fields: written }
                if !written.is_empty()
                    && written.len() == declared.len()
                    && shape(variant) == "record" =>
            {
                let fields = declared
                    .iter()
                    .zip(written)
                    .enumerate()
                    .map(|(position, (field, (name, declaration)))| {
                        if field.name != name {
                            return Err(unsupported(
                                "Named enum compiler/source field identity disagrees",
                            ));
                        }
                        let ty = self.tree(&field.ty, depth + 1)?;
                        Ok((
                            name.clone(),
                            Field {
                                position,
                                name,
                                declaration: declaration.into(),
                                ty,
                            },
                        ))
                    })
                    .collect::<Result<_, Failure>>()?;
                Ok(VariantShape::Named { fields })
            }
            _ => Err(unsupported(
                "Mixed enum loses its compiler unit/single tuple constructor shape",
            )),
        }
    }

    fn scalar_enum(
        &mut self,
        ty: &'a Type,
        declared: &'a [Declared],
        depth: usize,
    ) -> Result<TypeTree, Failure> {
        let reason = "Scalar enum requires local nongeneric destructor-free declarations";
        if !self.supports("rust.scalar-enum-values") {
            return Err(unsupported(reason));
        }
        let definition = self.local_enum(ty, reason)?;
        let source = self.declaration(definition)?;
        let declarations =
            crate::concrete_types::variant_declarations(&source).map_err(Failure::Unsupported)?;
        if declarations.len() != declared.len() {
            return Err(unsupported(
                "Enum source/resolved variant inventories disagree",
            ));
        }
        let variants = declared
            .iter()
            .zip(declarations)
            .enumerate()
            .map(|(position, (variant, written))| {
                self.scalar_variant((position, variant), written, depth)
            })
            .collect::<Result<Vec<_>, Failure>>()?;
        Ok(TypeTree::Enum {
            nominal: self.nominal(&definition.name, &source)?,
            source,
            variants,
        })
    }
}

impl<'a> Capture<'_, '_, 'a> {
    /// A scalar enum's variant: one bool or integer tuple payload.
    fn scalar_variant(
        &mut self,
        (position, variant): (usize, &'a Declared),
        (name, declaration): (String, String),
        depth: usize,
    ) -> Result<Variant, Failure> {
        let disagree = || unsupported("Enum source/resolved tuple payload inventories disagree");
        let [payload] = fields(variant) else {
            return Err(disagree());
        };
        if variant.name != name {
            return Err(disagree());
        }
        let payload_type = self.ty(&payload.ty)?;
        if !matches!(
            payload_type.kind,
            rstyle_core::cpg::model::TypeKind::Bool | rstyle_core::cpg::model::TypeKind::Integer
        ) {
            return Err(unsupported("Enum requires scalar bool/integer payloads"));
        }
        let ty = self.tree(&payload.ty, depth + 1)?;
        let payload = Field {
            position: 0,
            name: "0".into(),
            declaration: declaration.into(),
            ty,
        };
        Ok(Variant {
            position,
            name,
            payload,
        })
    }
}

fn fields(variant: &Declared) -> &[rstyle_core::cpg::model::TypeField] {
    variant.fields.as_deref().unwrap_or_default()
}

fn shape(variant: &Declared) -> &str {
    variant.shape.as_deref().unwrap_or_default()
}
