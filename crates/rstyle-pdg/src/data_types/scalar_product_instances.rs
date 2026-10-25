//! Canonical instance syntax only; guarded value authority remains separate.
use super::*;

pub(super) fn check(token: &TypeToken, depth: usize, entries: &mut usize) -> Result<(), String> {
    let TypeToken::ScalarProductInstance {
        nominal,
        argument,
        fields,
    } = token
    else {
        return Err("Expected a scalar product instance token".into());
    };
    let [(view_name, view), (value_name, value)] = fields.as_slice() else {
        return Err("Scalar product token requires exactly two fields".into());
    };
    if view_name != "0"
        || value_name != "1"
        || !shared_byte_slice(view)
        || !argument.matches_payload(value)
        || nominal
            .source_identity
            .as_ref()
            .is_none_or(|identity| identity.is_empty())
        || nominal.package_name.is_empty()
        || nominal.package_version.is_empty()
        || nominal.definition_path.is_empty()
        || crate::source_identity::Scheme::parse(&nominal.manifest_digest).is_none()
        || crate::source_identity::Scheme::parse(&nominal.declaration_digest).is_none()
    {
        return Err("Scalar product token changes its complete instance shape".into());
    }

    super::check_nominal_fields(fields, depth, entries)
}

pub(super) fn contains(token: &TypeToken) -> bool {
    match token {
        TypeToken::ScalarProductInstance { .. } => true,
        TypeToken::Tuple(values) => values.iter().any(contains),
        TypeToken::Result { ok, err } => contains(ok) || contains(err),
        TypeToken::Array { element, .. }
        | TypeToken::Shared(element)
        | TypeToken::Slice(element)
        | TypeToken::Option(element) => contains(element),
        TypeToken::Struct { fields, .. }
        | TypeToken::Enum {
            variants: fields, ..
        } => fields.iter().any(|(_, field)| contains(field)),
        TypeToken::MixedEnum { variants, .. } => variants
            .iter()
            .flat_map(|(_, shape)| shape.fields())
            .any(contains),
        _ => false,
    }
}
