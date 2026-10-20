//! Complete declaration validation for finite local scalar tuple variants.
use super::*;

pub fn variant_declarations(source: &str) -> Result<Vec<(String, String)>, String> {
    use syn::spanned::Spanned;
    let declaration: syn::ItemEnum = syn::parse_str(source).map_err(|error| error.to_string())?;
    if source.len() > MAX_SOURCE_BYTES
        || declaration.variants.is_empty()
        || declaration.variants.len() > 16
    {
        return Err("Scalar enum exceeds declaration/variant budget".into());
    }
    declaration
        .variants
        .iter()
        .map(|variant| {
            let syn::Fields::Unnamed(fields) = &variant.fields else {
                return Err("Scalar enum requires single tuple payloads".into());
            };
            if fields.unnamed.len() != 1 || variant.discriminant.is_some() {
                return Err(
                    "Scalar enum requires a single payload and implicit discriminants".into(),
                );
            }
            let span = fields.unnamed[0].ty.span();
            let spelling = source
                .get(byte_offset(source, span.start())..byte_offset(source, span.end()))
                .ok_or("Missing scalar enum payload source")?;
            Ok((variant.ident.to_string(), spelling.to_owned()))
        })
        .collect()
}

pub(super) fn check(
    nominal: &Nominal,
    source: &crate::SourceText,
    variants: &[Variant],
) -> Result<(), String> {
    if source.len() > MAX_SOURCE_BYTES || variants.is_empty() || variants.len() > 16 {
        return Err("Scalar enum exceeds declaration/variant budget".into());
    }

    check_identity(nominal, source)?;
    let declaration: syn::ItemEnum = syn::parse_str(source)
        .map_err(|error| format!("Invalid concrete enum declaration: {error}"))?;
    if nominal.definition_path.rsplit("::").next() != Some(declaration.ident.to_string().as_str())
        || !declaration.generics.params.is_empty()
        || declaration.generics.where_clause.is_some()
        || declaration.variants.len() != variants.len()
    {
        return Err("Concrete enum name, generics or variant inventory disagrees".into());
    }
    let mut names = std::collections::BTreeSet::new();
    for (position, (declared, captured)) in declaration.variants.iter().zip(variants).enumerate() {
        if !names.insert(&captured.name) {
            return Err("Concrete enum repeats a variant name".into());
        }
        check_variant(source, position, declared, captured)?;
    }
    Ok(())
}

fn check_variant(
    source: &str,
    position: usize,
    declared: &syn::Variant,
    captured: &Variant,
) -> Result<(), String> {
    let syn::Fields::Unnamed(fields) = &declared.fields else {
        return Err("Scalar enum requires single tuple payloads".into());
    };
    if captured.position != position
        || declared.ident != captured.name
        || declared.discriminant.is_some()
        || fields.unnamed.len() != 1
        || !matches!(&captured.payload.ty, TypeTree::Scalar(value) if scalar(value) && value != "unit")
    {
        return Err("Concrete enum variant or scalar payload disagrees".into());
    }
    check_field(source, 0, &fields.unnamed[0], &captured.payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed() -> TypeTree {
        let source: crate::SourceText = "enum Kind { Byte(u8), End(u16) }".into();
        TypeTree::Enum {
            nominal: Nominal {
                source_identity: Some("test-source".into()),
                package_name: "sample".into(),
                package_version: "1".into(),
                manifest_digest: digest(b"manifest"),
                definition_path: "sample::Kind".into(),
                declaration_digest: source.identity().into(),
            },
            source,
            variants: [("Byte", "u8"), ("End", "u16")]
                .into_iter()
                .enumerate()
                .map(|(position, (name, scalar))| Variant {
                    position,
                    name: name.into(),
                    payload: Field {
                        position: 0,
                        name: "0".into(),
                        declaration: scalar.into(),
                        ty: TypeTree::Scalar(scalar.into()),
                    },
                })
                .collect(),
        }
    }

    #[test]
    fn complete_scalar_variants_validate_without_authorizing_historical_policies() {
        let tree = observed();
        validate(&tree).unwrap();
        let token = crate::data_types::token(&tree).unwrap();
        assert_eq!(
            crate::data_types::parse(&token).unwrap(),
            crate::data_types::TypeToken::from_tree(&tree)
        );
        assert!(
            crate::data_types::parse_for(
                &token,
                crate::extraction_profile::ExtractionProfile::DataV2
            )
            .is_err()
        );
    }

    #[test]
    fn incomplete_reordered_and_wrong_payload_observations_are_rejected() {
        for mutation in 0..5 {
            let mut tree = observed();
            let TypeTree::Enum { variants, .. } = &mut tree else {
                unreachable!()
            };
            match mutation {
                0 => {
                    variants.pop();
                }
                1 => variants.swap(0, 1),
                2 => variants[1].name = "Byte".into(),
                3 => variants[1].payload.ty = TypeTree::Scalar("u8".into()),
                _ => {
                    variants[0].payload.ty =
                        TypeTree::Shared(Box::new(TypeTree::Scalar("u8".into())))
                }
            }
            assert!(validate(&tree).is_err(), "mutation {mutation}");
        }
    }

    #[test]
    fn unsupported_declared_shapes_cannot_supply_scalar_variant_observations() {
        for replacement in [
            "enum Kind { Byte { value: u8 }, End(u16) }",
            "enum Kind { Byte(u8, u8), End(u16) }",
            "enum Kind { Byte(u8) = 0, End(u16) = 1 }",
            "enum Kind<T> { Byte(T), End(u16) }",
            "enum Kind { Byte(&'static u8), End(u16) }",
            "enum Kind { Byte(u8), End }",
        ] {
            let mut tree = observed();
            let TypeTree::Enum {
                nominal, source, ..
            } = &mut tree
            else {
                unreachable!()
            };
            *source = replacement.into();
            nominal.declaration_digest = source.identity().into();
            assert!(validate(&tree).is_err(), "{replacement}");
        }
    }
}
