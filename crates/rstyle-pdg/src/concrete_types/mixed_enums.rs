//! Complete unit/single-tuple child declarations, without enum operation semantics.
use super::*;
use syn::spanned::Spanned;
mod payload;
pub(crate) use payload::closed_payload;

pub fn mixed_variant_declarations(
    source: &str,
) -> Result<Vec<(String, VariantShape<String>)>, String> {
    let declaration = declaration(source)?;
    declaration
        .variants
        .iter()
        .map(|variant| {
            let shape = match &variant.fields {
                syn::Fields::Unit => VariantShape::Unit {},
                syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    let span = fields.unnamed[0].ty.span();
                    let spelling = source
                        .get(byte_offset(source, span.start())..byte_offset(source, span.end()))
                        .ok_or("Missing mixed enum payload source")?;
                    VariantShape::SingleTuple {
                        field: spelling.into(),
                    }
                }
                _ => return Err("Mixed enum requires unit or single tuple variants".into()),
            };
            Ok((variant.ident.to_string(), shape))
        })
        .collect()
}

fn declaration(source: &str) -> Result<syn::ItemEnum, String> {
    let value: syn::ItemEnum = syn::parse_str(source).map_err(|error| error.to_string())?;
    if source.len() > MAX_SOURCE_BYTES
        || value.variants.is_empty()
        || value.variants.len() > 16
        || !value.generics.params.is_empty()
        || value.generics.where_clause.is_some()
        || value
            .variants
            .iter()
            .any(|variant| variant.discriminant.is_some())
        || !value
            .variants
            .iter()
            .any(|variant| matches!(variant.fields, syn::Fields::Unit))
        || !value
            .variants
            .iter()
            .any(|variant| !variant.fields.is_empty())
    {
        return Err(
            "Mixed enum requires bounded nongeneric unit/payload variants and implicit tags".into(),
        );
    }
    Ok(value)
}

pub(super) fn check(
    nominal: &Nominal,
    source: &crate::SourceText,
    variants: &[MixedVariant],
) -> Result<(), String> {
    check_identity(nominal, source)?;
    let declared = declaration(source)?;
    if nominal.definition_path.rsplit("::").next() != Some(declared.ident.to_string().as_str())
        || declared.variants.len() != variants.len()
    {
        return Err("Mixed enum name or complete variant inventory disagrees".into());
    }

    let mut names = std::collections::BTreeSet::new();
    for (position, (original, captured)) in declared.variants.iter().zip(variants).enumerate() {
        if captured.position != position
            || original.ident != captured.name
            || !names.insert(&captured.name)
        {
            return Err("Mixed enum variant identity/order disagrees".into());
        }
        check_variant(source, original, &captured.shape)?;
    }
    Ok(())
}

fn check_variant(
    source: &str,
    original: &syn::Variant,
    captured: &VariantShape<Field>,
) -> Result<(), String> {
    match (&original.fields, captured) {
        (syn::Fields::Unit, VariantShape::Unit {}) => Ok(()),
        (syn::Fields::Unnamed(fields), VariantShape::SingleTuple { field })
            if fields.unnamed.len() == 1 && closed_payload(&field.ty) =>
        {
            check_field(source, 0, &fields.unnamed[0], field)
        }
        _ => Err("Mixed enum loses its exact unit/single tuple shape or closed payload".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed() -> TypeTree {
        let source: crate::SourceText = "enum Anchored { No, Yes, Pattern(u32) }".into();
        TypeTree::MixedEnum {
            nominal: Nominal {
                source_identity: Some("source".into()),
                package_name: "sample".into(),
                package_version: "1".into(),
                manifest_digest: digest(b"manifest"),
                definition_path: "sample::Anchored".into(),
                declaration_digest: source.identity().into(),
            },
            source,
            variants: ["No", "Yes", "Pattern"]
                .into_iter()
                .enumerate()
                .map(|(position, name)| MixedVariant {
                    position,
                    name: name.into(),
                    shape: if position < 2 {
                        VariantShape::Unit {}
                    } else {
                        VariantShape::SingleTuple {
                            field: Field {
                                position: 0,
                                name: "0".into(),
                                declaration: "u32".into(),
                                ty: TypeTree::Scalar("u32".into()),
                            },
                        }
                    },
                })
                .collect(),
        }
    }

    #[test]
    fn mixed_shapes_are_complete_observations_without_owned_value_permission() {
        let tree = observed();
        validate(&tree).unwrap();
        let text = crate::data_types::token(&tree).unwrap();
        assert_eq!(
            crate::data_types::parse(&text).unwrap(),
            crate::data_types::TypeToken::from_tree(&tree)
        );
        for profile in [
            crate::extraction_profile::ExtractionProfile::DataV2,
            crate::extraction_profile::ExtractionProfile::DataV3,
        ] {
            assert!(crate::data_types::parse_for(&text, profile).is_err());
        }
        for shape in [
            serde_json::json!({}),
            serde_json::json!({"kind":"SingleTuple"}),
            serde_json::json!({"kind":"Unit", "field":null}),
        ] {
            assert!(serde_json::from_value::<VariantShape<Field>>(shape).is_err());
        }
    }

    #[test]
    fn mixed_children_cross_only_a_shared_struct_boundary() {
        use crate::data_types::{TypeToken, parse_for};
        use crate::extraction_profile::ExtractionProfile::{DataV2, DataV3};
        let child = TypeToken::from_tree(&observed());
        let TypeToken::MixedEnum { nominal, .. } = &child else {
            unreachable!()
        };
        let owner = TypeToken::Struct {
            nominal: nominal.clone(),
            fields: vec![("anchored".into(), child.clone())],
        };
        let shared = TypeToken::Shared(Box::new(owner.clone()));
        assert!(parse_for(&shared.text().unwrap(), DataV3).is_ok());
        assert!(parse_for(&shared.text().unwrap(), DataV2).is_err());
        for value in [
            owner,
            child.clone(),
            TypeToken::Shared(Box::new(child.clone())),
            TypeToken::Tuple(vec![shared, child]),
        ] {
            assert!(parse_for(&value.text().unwrap(), DataV3).is_err());
        }
    }

    #[test]
    fn missing_reordered_or_forged_mixed_shapes_reject() {
        for mutation in 0..6 {
            let mut tree = observed();
            let TypeTree::MixedEnum { variants, .. } = &mut tree else {
                unreachable!()
            };
            match mutation {
                0 => {
                    variants.remove(1);
                }
                1 => variants.swap(0, 1),
                2 => variants[1].name = "No".into(),
                3 => variants[2].shape = VariantShape::Unit {},
                4 => variants[0].shape = variants[2].shape.clone(),
                _ => {
                    let VariantShape::SingleTuple { field } = &mut variants[2].shape else {
                        unreachable!()
                    };
                    field.ty = TypeTree::Scalar("u64".into());
                }
            }
            assert!(validate(&tree).is_err(), "mutation {mutation}");
        }
    }

    #[test]
    fn unsupported_source_shapes_remain_distinct_from_unit_variants() {
        for source in [
            "enum E { A, B() }",
            "enum E { A, B(u8,u8) }",
            "enum E { A, B { x: u8 } }",
            "enum E { A = 2, B(u8) }",
            "enum E<T> { A, B(T) }",
            "enum E { A, B }",
            "enum E { A(u8), B(u8) }",
        ] {
            assert!(mixed_variant_declarations(source).is_err(), "{source}");
        }
    }
}
