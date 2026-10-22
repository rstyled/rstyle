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
                syn::Fields::Named(fields)
                    if !fields.named.is_empty() && fields.named.len() <= MAX_FIELDS =>
                {
                    VariantShape::Named {
                        fields: fields
                            .named
                            .iter()
                            .map(|field| named_declaration(source, field))
                            .collect::<Result<_, String>>()?,
                    }
                }
                _ => {
                    return Err(
                        "Mixed enum requires bounded unit/single tuple/named variants".into(),
                    );
                }
            };
            Ok((variant.ident.to_string(), shape))
        })
        .collect()
}

fn declaration(source: &str) -> Result<syn::ItemEnum, String> {
    let value: syn::ItemEnum = syn::parse_str(source).map_err(|error| error.to_string())?;
    if source.len() > MAX_SOURCE_BYTES
        || value.variants.is_empty()
        || value.variants.len() > MAX_VARIANTS
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
        (syn::Fields::Named(fields), VariantShape::Named { fields: captured })
            if !fields.named.is_empty()
                && fields.named.len() == captured.len()
                && captured.len() <= MAX_FIELDS =>
        {
            for (position, (original, (name, field))) in
                fields.named.iter().zip(captured).enumerate()
            {
                if original.ident.as_ref().map(ToString::to_string).as_ref() != Some(name)
                    || field.name != *name
                    || !matches!(&field.ty, TypeTree::Scalar(value) if scalar(value) && value != "unit")
                {
                    return Err("Named enum field identity or scalar payload disagrees".into());
                }
                check_field(source, position, original, field)?;
            }
            Ok(())
        }
        _ => Err("Mixed enum loses its exact variant shape or closed payload".into()),
    }
}

fn named_declaration(source: &str, field: &syn::Field) -> Result<(String, String), String> {
    let name = field
        .ident
        .as_ref()
        .ok_or("Missing named enum field")?
        .to_string();
    let span = field.ty.span();
    let spelling = source
        .get(byte_offset(source, span.start())..byte_offset(source, span.end()))
        .ok_or("Missing named enum field source")?;

    Ok((name, spelling.into()))
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

    fn named_error() -> TypeTree {
        let source: crate::SourceText = "#[derive(Debug)] #[non_exhaustive] enum DecompressError { OutputTooSmall { expected: usize, actual: usize }, LiteralOutOfBounds, ExpectedAnotherByte, OffsetZero, OffsetOutOfBounds }".into();
        let TypeTree::MixedEnum { mut nominal, .. } = observed() else {
            unreachable!()
        };
        nominal.definition_path = "sample::DecompressError".into();
        nominal.declaration_digest = source.identity().into();
        let variants = [
            "OutputTooSmall",
            "LiteralOutOfBounds",
            "ExpectedAnotherByte",
            "OffsetZero",
            "OffsetOutOfBounds",
        ]
        .into_iter()
        .enumerate()
        .map(|(position, name)| MixedVariant {
            position,
            name: name.into(),
            shape: if position == 0 {
                VariantShape::Named {
                    fields: ["expected", "actual"]
                        .into_iter()
                        .enumerate()
                        .map(|(position, name)| {
                            (
                                name.into(),
                                Field {
                                    position,
                                    name: name.into(),
                                    declaration: "usize".into(),
                                    ty: TypeTree::Scalar("usize".into()),
                                },
                            )
                        })
                        .collect(),
                }
            } else {
                VariantShape::Unit {}
            },
        })
        .collect();
        TypeTree::MixedEnum {
            nominal,
            source,
            variants,
        }
    }

    #[test]
    fn named_inventory_retains_every_field_and_historical_ownership_boundary() {
        let tree = named_error();
        validate(&tree).unwrap();
        assert_eq!(tree.nominal_fields().count(), 2);
        let token = crate::data_types::TypeToken::from_tree(&tree);
        assert!(token.native_width());
        let text = token.text().unwrap();
        assert!(text.contains("expected") && text.contains("actual"));
        assert!(
            crate::data_types::parse_for(
                &text,
                crate::extraction_profile::ExtractionProfile::DataV2
            )
            .is_err()
        );
        assert_eq!(
            crate::data_types::parse_for(
                &text,
                crate::extraction_profile::ExtractionProfile::DataV3
            )
            .unwrap(),
            token
        );
        let wire = serde_json::to_vec(&tree).unwrap();
        assert_eq!(serde_json::from_slice::<TypeTree>(&wire).unwrap(), tree);
    }

    #[test]
    fn named_inventory_tampering_rejects() {
        for mutation in 0..7 {
            let mut tree = named_error();
            let TypeTree::MixedEnum { variants, .. } = &mut tree else {
                unreachable!()
            };
            if mutation == 0 {
                variants.remove(4);
            } else {
                let VariantShape::Named { fields } = &mut variants[0].shape else {
                    unreachable!()
                };
                match mutation {
                    1 => {
                        fields.pop();
                    }
                    2 => fields.swap(0, 1),
                    3 => fields[0].0 = "other".into(),
                    4 => fields[0].1.name = "actual".into(),
                    5 => fields[0].1.ty = TypeTree::Scalar("u64".into()),
                    _ => fields[0].1.position = 1,
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
            "enum E { A, B {} }",
            "enum E { A = 2, B(u8) }",
            "enum E<T> { A, B(T) }",
            "enum E { A, B }",
            "enum E { A(u8), B(u8) }",
        ] {
            assert!(mixed_variant_declarations(source).is_err(), "{source}");
        }
    }
}
