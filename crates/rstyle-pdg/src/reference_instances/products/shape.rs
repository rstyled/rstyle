//! The entire mixed product remains present, including its nonselected payload.
use super::{Result, TypeToken};
use crate::concrete_types::VariantShape;

pub(super) fn verify(result: &TypeToken) -> Result<()> {
    let input_fields = fields(
        result,
        "util::search::Input",
        &["haystack", "span", "anchored", "earliest"],
    )?;
    let expected = TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
        "u8".into(),
    )))));
    if input_fields[0].1 != expected || input_fields[3].1 != TypeToken::Scalar("bool".into()) {
        return Err("Checked Input product changes its view/earliest components".into());
    }

    let endpoints = fields(&input_fields[1].1, "util::search::Span", &["start", "end"])?;
    if endpoints
        .iter()
        .any(|(_, ty)| *ty != TypeToken::Scalar("usize".into()))
    {
        return Err("Checked Span product changes its complete endpoint types".into());
    }

    anchored(&input_fields[2].1)
}

fn fields<'a>(ty: &'a TypeToken, path: &str, names: &[&str]) -> Result<&'a [(String, TypeToken)]> {
    let TypeToken::Struct { nominal, fields } = ty else {
        return Err("Checked reference product omits a complete nominal struct".into());
    };
    if nominal.definition_path != path
        || !fields
            .iter()
            .map(|(name, _)| name.as_str())
            .eq(names.iter().copied())
    {
        return Err("Checked reference product changes nominal field identity/order".into());
    }
    Ok(fields)
}

fn anchored(ty: &TypeToken) -> Result<()> {
    let TypeToken::MixedEnum { nominal, variants } = ty else {
        return Err("Anchored must retain its complete mixed-enum shape".into());
    };
    let [
        (no, VariantShape::Unit {}),
        (yes, VariantShape::Unit {}),
        (pattern, VariantShape::SingleTuple { field }),
    ] = variants.as_slice()
    else {
        return Err("Anchored must retain both unit variants and its exact Pattern payload".into());
    };
    if nominal.definition_path != "util::search::Anchored"
        || no != "No"
        || yes != "Yes"
        || pattern != "Pattern"
    {
        return Err("Checked Anchored changes original nominal/variant identities".into());
    }

    let pattern = fields(field, "util::primitives::PatternID", &["0"])?;
    let index = fields(&pattern[0].1, "util::primitives::SmallIndex", &["0"])?;
    if index[0].1 != TypeToken::Scalar("u32".into()) {
        return Err(
            "Anchored Pattern payload loses its complete scalar-backed no-drop product".into(),
        );
    }

    Ok(())
}

pub(super) fn contains(root: &TypeToken, expected: &TypeToken) -> bool {
    if root == expected {
        return true;
    }
    match root {
        TypeToken::Struct { fields, .. } => fields.iter().any(|(_, ty)| contains(ty, expected)),
        TypeToken::MixedEnum { variants, .. } => variants
            .iter()
            .any(|(_, shape)| shape.field().is_some_and(|ty| contains(ty, expected))),
        TypeToken::Shared(inner) | TypeToken::Slice(inner) => contains(inner, expected),
        _ => false,
    }
}
