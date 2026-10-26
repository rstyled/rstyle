//! Mixed enum shapes are type evidence for borrowed owners, not enum operations.
use super::*;

pub(super) fn token(
    nominal: &Nominal,
    variants: &[crate::concrete_types::MixedVariant],
) -> TypeToken {
    TypeToken::MixedEnum {
        nominal: nominal.clone(),
        variants: named_tokens(
            variants,
            |variant| &variant.name,
            |variant| variant.shape.map(|field| TypeToken::from_tree(&field.ty)),
        ),
    }
}

pub(super) fn check(
    nominal: &Nominal,
    variants: &[(String, VariantShape<TypeToken>)],
    depth: usize,
    entries: &mut usize,
) -> Result<(), String> {
    let mut names = std::collections::BTreeSet::new();
    if !nominal
        .source_identity
        .as_ref()
        .is_some_and(|id| !id.is_empty())
        || variants.is_empty()
        || variants.len() > crate::concrete_types::MAX_VARIANTS
        || !variants
            .iter()
            .any(|(_, shape)| matches!(shape, VariantShape::Unit {} | VariantShape::Named { .. }))
        || !variants.iter().any(|(_, shape)| {
            matches!(
                shape,
                VariantShape::SingleTuple { .. } | VariantShape::Named { .. }
            )
        })
    {
        return Err("Mixed enum requires complete bounded source/payload evidence".into());
    }
    for (name, shape) in variants {
        if name.is_empty() || !names.insert(name) {
            return Err("Mixed enum variant identity is absent or repeated".into());
        }
        if let VariantShape::Named { fields } = shape {
            let mut names = std::collections::BTreeSet::new();
            if fields.is_empty()
                || fields.len() > crate::concrete_types::MAX_FIELDS
                || fields
                    .iter()
                    .any(|(name, _)| name.is_empty() || !names.insert(name))
            {
                return Err("Named enum requires unique ordered fields".into());
            }
        }
        for field in shape.fields() {
            if !crate::concrete_types::closed_payload(field) {
                return Err("Mixed enum requires closed scalar/struct payloads".into());
            }
            super::check(field, depth + 1, entries)?;
        }
    }
    Ok(())
}

/// New observations must not acquire the older unit/scalar ownership permission.
pub(super) fn requires_payload_evidence(ty: &TypeToken) -> bool {
    match ty {
        TypeToken::MixedEnum { variants, .. } => {
            !variants.iter().any(|(_, shape)| matches!(shape, VariantShape::Unit {}))
                || variants.iter().any(|(_, shape)| {
                    matches!(shape, VariantShape::Named { fields } if fields.iter().any(|(_, value)| !matches!(value, TypeToken::Scalar(_))))
                        || shape.fields().any(requires_payload_evidence)
                })
        }
        TypeToken::Struct { fields, .. } | TypeToken::Enum { variants: fields, .. } => {
            fields.iter().any(|(_, field)| requires_payload_evidence(field))
        }
        TypeToken::Tuple(values) => values.iter().any(requires_payload_evidence),
        TypeToken::Result { ok, err } => {
            crate::concrete_types::payload_errors::GRAMMAR.permits(ok, err)
                || requires_payload_evidence(ok) || requires_payload_evidence(err)
        }
        TypeToken::Shared(value) | TypeToken::Slice(value) | TypeToken::Option(value)
        | TypeToken::Array { element: value, .. } => requires_payload_evidence(value),
        _ => false,
    }
}

pub(super) fn borrowed_use(ty: &TypeToken) -> bool {
    match ty {
        TypeToken::Shared(inner) if matches!(inner.as_ref(), TypeToken::Struct { .. }) => true,
        TypeToken::MixedEnum { .. } => false,
        TypeToken::Struct { fields, .. } => fields.iter().all(|(_, field)| borrowed_use(field)),
        TypeToken::Tuple(values) => values.iter().all(borrowed_use),
        TypeToken::Result { ok, err } => borrowed_use(ok) && borrowed_use(err),
        TypeToken::Shared(inner)
        | TypeToken::Slice(inner)
        | TypeToken::Array { element: inner, .. }
        | TypeToken::Option(inner) => borrowed_use(inner),
        _ => true,
    }
}

/// Observing a reference-bearing variant never authorizes its tag or payload.
pub(super) fn reference_payload(ty: &TypeToken) -> bool {
    match ty {
        TypeToken::MixedEnum { .. } if crate::data_origins::shared(ty) => true,
        TypeToken::Result { ok, err } => reference_payload(ok) || reference_payload(err),
        TypeToken::Tuple(values) => values.iter().any(reference_payload),
        TypeToken::Struct { fields, .. }
        | TypeToken::Enum {
            variants: fields, ..
        } => fields.iter().any(|(_, field)| reference_payload(field)),
        TypeToken::Shared(value)
        | TypeToken::Slice(value)
        | TypeToken::Array { element: value, .. }
        | TypeToken::Option(value) => reference_payload(value),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(payload: TypeToken, unit: bool) -> TypeToken {
        let nominal = Nominal {
            source_identity: Some("source".into()),
            package_name: "sample".into(),
            package_version: "1".into(),
            manifest_digest: crate::source_identity::checksum(b"manifest"),
            definition_path: "sample::Error".into(),
            declaration_digest: crate::source_identity::checksum(b"declaration"),
        };
        let mut variants = vec![(
            "Value".into(),
            VariantShape::Named {
                fields: vec![("value".into(), payload)],
            },
        )];
        if unit {
            variants.push(("Empty".into(), VariantShape::Unit {}));
        }
        TypeToken::MixedEnum { nominal, variants }
    }

    #[test]
    fn diagnostic_payload_shapes_never_inherit_historical_owned_or_borrowed_permission() {
        for (payload, unit) in [
            (TypeToken::Scalar("usize".into()), false),
            (TypeToken::Shared(Box::new(TypeToken::Str)), true),
        ] {
            let ty = error(payload, unit);
            let TypeToken::MixedEnum { nominal, .. } = &ty else {
                unreachable!()
            };
            let owner = TypeToken::Struct {
                nominal: nominal.clone(),
                fields: vec![("0".into(), ty.clone())],
            };
            let result = TypeToken::Result {
                ok: Box::new(TypeToken::Scalar("usize".into())),
                err: Box::new(owner.clone()),
            };
            for ty in [
                ty,
                owner.clone(),
                TypeToken::Shared(Box::new(owner)),
                result,
            ] {
                let wire = ty.text().unwrap();
                assert_eq!(parse(&wire).unwrap(), ty);
                for profile in [
                    crate::extraction_profile::ExtractionProfile::DataV2,
                    crate::extraction_profile::ExtractionProfile::DataV3,
                ] {
                    assert!(parse_for(&wire, profile).is_err());
                }
            }
        }
    }
}
