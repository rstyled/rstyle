//! Reconstruct every observed mixed child, retaining explicit unit/tuple shape.
use super::*;
use crate::concrete_types::{Field, MixedVariant, VariantShape};

pub(super) fn collect(
    nominal: &Nominal,
    variants: &[(String, VariantShape<TypeToken>)],
    token: &TypeToken,
    tokens: &mut BTreeMap<String, TypeToken>,
) -> Result<(), String> {
    if let Some(previous) = tokens.insert(nominal.definition_path.clone(), token.clone()) {
        return if previous == *token {
            Ok(())
        } else {
            Err("Conflicting mixed body declaration".into())
        };
    }
    for field in variants.iter().filter_map(|(_, shape)| shape.field()) {
        collect_tokens(field, tokens)?;
    }
    Ok(())
}

pub(super) fn tree(
    nominal: &Nominal,
    variants: &[(String, VariantShape<TypeToken>)],
    origins: &Origins,
) -> Result<TypeTree, String> {
    let source = origin_source(nominal, origins)?;
    let declared = crate::concrete_types::mixed_variant_declarations(source)?;
    if declared.len() != variants.len() {
        return Err("Mixed body variant inventory disagrees".into());
    }

    let variants = declared
        .into_iter()
        .zip(variants)
        .enumerate()
        .map(|(position, ((name, source), (observed, value)))| {
            if name != *observed {
                return Err("Mixed body variant identity/order disagrees".into());
            }
            let shape = match (source, value) {
                (VariantShape::Unit {}, VariantShape::Unit {}) => VariantShape::Unit {},
                (
                    VariantShape::SingleTuple { field: declaration },
                    VariantShape::SingleTuple { field },
                ) => VariantShape::SingleTuple {
                    field: Field {
                        position: 0,
                        name: "0".into(),
                        declaration: declaration.into(),
                        ty: super::tree(field, origins)?,
                    },
                },
                _ => return Err("Mixed body variant shape disagrees".into()),
            };
            Ok(MixedVariant {
                position,
                name,
                shape,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(TypeTree::MixedEnum {
        nominal: nominal.clone(),
        source: source.into(),
        variants,
    })
}
