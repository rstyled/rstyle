//! Deterministic local data interface vocabulary, distinct from scalar compat.

use crate::concrete_types::{Nominal, TypeTree, scalar};
use serde::{Deserialize, Serialize};
mod converted;
mod result_grammar;
pub(crate) use result_grammar::ResultGrammar;
mod mapped_conversions;
mod mixed_owners;
mod parsed;
mod scalar_product_instances;
mod scalar_slices;
pub(crate) use scalar_slices::shared_byte_slice;
#[cfg(test)]
mod mapped_int_results;
#[cfg(test)]
mod range_values;
use crate::concrete_types::VariantShape;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TypeToken {
    Scalar(String),
    NonZero(crate::nonzero_values::Observation),
    Tuple(Vec<TypeToken>),
    Array {
        element: Box<TypeToken>,
        length: u64,
    },
    Shared(Box<TypeToken>),
    MutableScalar(String),
    Slice(Box<TypeToken>),
    Str,
    Option(Box<TypeToken>),
    Result {
        ok: Box<TypeToken>,
        err: Box<TypeToken>,
    },
    IntConversionResult(String),
    MappedIntConversionResult {
        ok: Box<TypeToken>,
    },
    RangeUsize,
    RangeToUsize,
    RangeFromUsize,
    SliceArrayResult {
        length: u64,
        borrowed: bool,
    },
    Struct {
        nominal: Nominal,
        fields: Vec<(String, TypeToken)>,
    },
    ScalarProductInstance {
        nominal: Nominal,
        argument: crate::scalar_product_instances::Argument,
        fields: Vec<(String, TypeToken)>,
    },
    FieldlessEnum {
        nominal: Nominal,
        variants: Vec<(String, i64)>,
    },
    Enum {
        nominal: Nominal,
        variants: Vec<(String, TypeToken)>,
    },
    MixedEnum {
        nominal: Nominal,
        variants: Vec<(String, VariantShape<TypeToken>)>,
    },
}

impl TypeToken {
    pub fn from_tree(tree: &TypeTree) -> Self {
        match tree {
            TypeTree::Scalar(value) => Self::Scalar(value.clone()),
            TypeTree::NonZero(observation) => Self::NonZero(observation.clone()),
            TypeTree::Tuple(values) => Self::Tuple(values.iter().map(Self::from_tree).collect()),
            TypeTree::Array { element, length } => Self::Array {
                element: Box::new(Self::from_tree(element)),
                length: *length,
            },
            TypeTree::Shared(value) => Self::Shared(Box::new(Self::from_tree(value))),
            TypeTree::MutableScalar(value) => Self::MutableScalar(value.clone()),
            TypeTree::Slice(value) => Self::Slice(Box::new(Self::from_tree(value))),
            TypeTree::Str => Self::Str,
            TypeTree::IntConversionResult(value) => Self::IntConversionResult(value.clone()),
            TypeTree::MappedIntConversionResult { ok } => Self::MappedIntConversionResult {
                ok: Box::new(Self::from_tree(ok)),
            },
            TypeTree::RangeUsize { .. } => Self::RangeUsize,
            TypeTree::RangeToUsize { .. } => Self::RangeToUsize,
            TypeTree::RangeFromUsize { .. } => Self::RangeFromUsize,
            TypeTree::Option(value) => Self::Option(Box::new(Self::from_tree(value))),
            TypeTree::Result { ok, err } => Self::from_result(ok, err),
            TypeTree::SliceArrayResult { length, borrowed } => Self::SliceArrayResult {
                length: *length,
                borrowed: *borrowed,
            },
            TypeTree::ScalarProductInstance(instance) => Self::ScalarProductInstance {
                nominal: instance.nominal.clone(),
                argument: instance.argument,
                fields: named_tokens(
                    &instance.fields,
                    |field| &field.name,
                    |field| Self::from_tree(&field.ty),
                ),
            },
            TypeTree::Struct { .. }
            | TypeTree::GeneratedStruct { .. }
            | TypeTree::FieldlessEnum { .. }
            | TypeTree::MixedEnum { .. }
            | TypeTree::Enum { .. } => Self::from_nominal(tree),
        }
    }

    fn from_nominal(tree: &TypeTree) -> Self {
        match tree {
            TypeTree::Struct {
                nominal, fields, ..
            }
            | TypeTree::GeneratedStruct {
                nominal, fields, ..
            } => Self::Struct {
                nominal: nominal.clone(),
                fields: named_tokens(
                    fields,
                    |field| &field.name,
                    |field| Self::from_tree(&field.ty),
                ),
            },
            TypeTree::FieldlessEnum {
                nominal, variants, ..
            } => Self::FieldlessEnum {
                nominal: nominal.clone(),
                variants: variants.clone(),
            },
            TypeTree::MixedEnum {
                nominal, variants, ..
            } => mixed_owners::token(nominal, variants),
            TypeTree::Enum {
                nominal, variants, ..
            } => Self::Enum {
                nominal: nominal.clone(),
                variants: named_tokens(
                    variants,
                    |variant| &variant.name,
                    |variant| Self::from_tree(&variant.payload.ty),
                ),
            },
            _ => unreachable!("Nominal conversion requires a nominal tree"),
        }
    }

    fn from_result(ok: &TypeTree, err: &TypeTree) -> Self {
        Self::Result {
            ok: Box::new(Self::from_tree(ok)),
            err: Box::new(Self::from_tree(err)),
        }
    }

    pub fn text(&self) -> Result<String, String> {
        if let Self::Scalar(value) = self {
            return Ok(value.clone());
        }
        serde_json::to_string(self).map_err(|error| error.to_string())
    }

    pub fn native_width(&self) -> bool {
        match self {
            Self::Scalar(value) | Self::IntConversionResult(value) => {
                matches!(value.as_str(), "isize" | "usize")
            }
            Self::MutableScalar(_)
            | Self::FieldlessEnum { .. }
            | Self::RangeUsize
            | Self::RangeToUsize
            | Self::RangeFromUsize => true,
            Self::Tuple(values) => values.iter().any(Self::native_width),
            Self::Result { ok, err } => ok.native_width() || err.native_width(),
            Self::Array { element, .. }
            | Self::Shared(element)
            | Self::Slice(element)
            | Self::Option(element)
            | Self::MappedIntConversionResult { ok: element } => element.native_width(),
            Self::Struct { fields, .. } | Self::ScalarProductInstance { fields, .. } => {
                fields.iter().any(|(_, field)| field.native_width())
            }
            Self::Enum { variants, .. } => {
                variants.iter().any(|(_, payload)| payload.native_width())
            }
            Self::MixedEnum { variants, .. } => variants
                .iter()
                .flat_map(|(_, shape)| shape.fields())
                .any(Self::native_width),
            _ => false,
        }
    }
}

/// Preserve declaration order and names while converting concrete member evidence.
fn named_tokens<T, U>(
    members: &[T],
    name: impl Fn(&T) -> &str,
    token: impl Fn(&T) -> U,
) -> Vec<(String, U)> {
    members
        .iter()
        .map(|member| (name(member).to_owned(), token(member)))
        .collect()
}

pub(crate) fn concrete_text(tree: &TypeTree) -> String {
    TypeToken::from_tree(tree)
        .text()
        .expect("checked concrete type")
}

pub fn token(tree: &TypeTree) -> Result<String, String> {
    converted::get(tree, || checked_token(tree))
}

/// A successful exact conversion has already checked this declaration shape.
/// Callers must still count every occurrence and check contextual depth limits.
pub(crate) fn has_checked_shape(tree: &TypeTree) -> bool {
    converted::contains(tree)
}

fn checked_token(tree: &TypeTree) -> Result<String, String> {
    crate::concrete_types::validate(tree)?;
    let text = TypeToken::from_tree(tree).text()?;
    if text.len() > crate::concrete_types::MAX_SOURCE_BYTES {
        return Err("Data interface token exceeds byte budget".into());
    }
    Ok(text)
}

pub fn parse(text: &str) -> Result<TypeToken, String> {
    if scalar(text) || matches!(text, "char" | "f32" | "f64") {
        return Ok(TypeToken::Scalar(text.into()));
    }
    if text.len() > crate::concrete_types::MAX_SOURCE_BYTES {
        return Err("Data interface token exceeds byte budget".into());
    }

    parsed::get(text, || parse_canonical(text))
}

fn parse_canonical(text: &str) -> Result<TypeToken, String> {
    let token: TypeToken = serde_json::from_str(text).map_err(|error| error.to_string())?;
    check(&token, 0, &mut 0)?;
    if token.text()? != text {
        return Err("Noncanonical data interface serialization".into());
    }
    Ok(token)
}

pub(crate) fn parse_for(
    text: &str,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Result<TypeToken, String> {
    parse_context(text, profile, false)
}

pub(crate) fn parse_in(
    text: &str,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<TypeToken, String> {
    if crate::named_error_payloads::permits_type(text, facts)?
        || crate::nonzero_digit_calls::permits_type(text, facts)?
        || crate::scalar_product_instances::forwarders::permits_type(text, facts)?
        || crate::scalar_product_instances::activation::permits_type(text, facts)?
        || crate::copy_tagged_values::call_wrappers::permits_type(text, facts)?
        || crate::copy_tagged_values::permits_type(text, facts)?
    {
        return parse(text);
    }
    let permitted = facts.reference_instance().is_some_and(|instance| {
        crate::extraction_profile::facts_support(
            facts,
            crate::reference_instances::activation::CAPABILITY,
        ) && parse(text).is_ok_and(|ty| instance.products().permits(&ty))
    });
    let permitted = permitted
        || facts.reference_forwarder().is_some_and(|instance| {
            crate::extraction_profile::facts_support(
                facts,
                crate::reference_instances::forwarder_activation::CAPABILITY,
            ) && parse(text).is_ok_and(|ty| instance.constructor().products().permits(&ty))
        });
    parse_context(
        text,
        crate::extraction_profile::ExtractionProfile::from_facts(facts)?,
        permitted,
    )
}

fn parse_context(
    text: &str,
    profile: crate::extraction_profile::ExtractionProfile,
    checked_product: bool,
) -> Result<TypeToken, String> {
    let ty = parse(text)?;
    if matches!(&ty, TypeToken::MutableScalar(_))
        && !profile.supports("rust.mutable-scalar-formals")
    {
        return Err("Mutable scalar formals require their registered capability".into());
    }
    check_fresh_capabilities(text, profile)?;
    check_checked_capabilities(&ty, profile)?;

    check_guarded_value_evidence(&ty)?;

    if text.contains("\"kind\":\"mixed_enum\"")
        && !checked_product
        && (!profile.supports("rust.borrowed-mixed-owners")
            || !(mixed_owners::borrowed_use(&ty)
                || profile.supports(crate::borrowed_result_types::CAPABILITY)
                    && crate::borrowed_result_types::owned_use(&ty)))
    {
        return Err(
            "Mixed enums require complete borrowed owners, without direct enum value operations"
                .into(),
        );
    }
    if matches!(text, "f32" | "f64") && !profile.supports(crate::float_scalars::CAPABILITY) {
        return Err("Float scalars require their registered capability".into());
    }
    if text == "char" && !profile.supports(crate::character_scalars::CAPABILITY) {
        return Err("Character scalars require their registered capability".into());
    }
    if text.contains("\"kind\":\"fieldless_enum\"")
        && !profile.supports("rust.fieldless-enum-values")
    {
        return Err("Fieldless enum requires its registered capability".into());
    }
    if text.contains("\"kind\":\"enum\"") && !profile.supports("rust.scalar-enum-values") {
        return Err("Enum type requires its registered scalar payload capability".into());
    }
    if text.contains("\"kind\":\"option\"")
        && (!profile.supports("rust.checked-slice-option")
            || scalar_option(&ty) && !profile.supports("rust.scalar-option-values"))
    {
        return Err("Option type requires its registered payload capability".into());
    }

    check_conversion_capabilities(text, profile)?;
    Ok(ty)
}

fn check_guarded_value_evidence(ty: &TypeToken) -> Result<(), String> {
    if mapped_conversions::contains(ty) {
        return Err(
            "Mapped integer Results require unavailable complete callback value evidence".into(),
        );
    }
    if mixed_owners::requires_payload_evidence(ty) {
        return Err(
            "Named enum payloads require unavailable complete ownership and guarded value evidence"
                .into(),
        );
    }
    if scalar_product_instances::contains(ty) {
        return Err("Scalar product values require exact guarded instance evidence".into());
    }
    if mixed_owners::reference_payload(ty) {
        return Err(
            "Mixed enum reference payloads require unavailable guarded value evidence".into(),
        );
    }
    if crate::concrete_types::static_string_results::GRAMMAR.contains(ty) {
        return Err(
            "Static string product Results require unavailable guarded value evidence".into(),
        );
    }

    Ok(())
}

fn check_checked_capabilities(
    ty: &TypeToken,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Result<(), String> {
    if crate::result_maps::types::contains_result(ty)
        && !profile.supports(crate::result_maps::CAPABILITY)
    {
        return Err("Constructor Result maps require their registered capability".into());
    }
    if crate::concrete_results::fresh_products::used(ty)
        && !profile.supports(crate::concrete_results::fresh_products::CAPABILITY)
    {
        return Err("Fresh product Result requires its registered capability".into());
    }
    if crate::concrete_results::scalar_errors::contains_result(ty)
        && !profile.supports(crate::concrete_results::scalar_errors::CAPABILITY)
    {
        return Err("Fresh scalar-error Result requires its registered capability".into());
    }
    if crate::concrete_results::copy_errors::contains_copy_result(ty)
        && !profile.supports(crate::concrete_results::copy_errors::CAPABILITY)
    {
        return Err("Copy fieldless Result requires its registered capability".into());
    }
    if (contains_option(ty, &shared_byte_slice) || crate::borrowed_result_types::uses_result(ty))
        && !profile.supports("rust.checked-borrowed-result-flow")
    {
        return Err("Borrowed checked values require their checked-result capability".into());
    }
    if product_option(ty) && !profile.supports("rust.concrete-option-propagation") {
        return Err("Product Option requires its registered propagation capability".into());
    }

    Ok(())
}

fn check_fresh_capabilities(
    text: &str,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Result<(), String> {
    if text.contains("\"kind\":\"non_zero\"") {
        return Err("NonZero values require a complete checked composition contract".into());
    }
    if text.contains("\"kind\":\"range_to_usize\"")
        || text.contains("\"kind\":\"range_from_usize\"")
    {
        return Err("Checked range observation grants no standalone value permission".into());
    }

    if text.contains("\"kind\":\"result\"") && !profile.supports("rust.fresh-concrete-results") {
        return Err("Concrete Result requires its registered fresh-result capability".into());
    }
    if text.contains("\"kind\":\"range_usize\"") && !profile.supports("rust.fresh-range-results") {
        return Err("Range results require their registered capability".into());
    }
    Ok(())
}

pub(crate) fn scalar_option(token: &TypeToken) -> bool {
    contains_option(token, &|payload| matches!(payload, TypeToken::Scalar(_)))
}

pub(crate) fn product_option(token: &TypeToken) -> bool {
    contains_option(token, &|payload| {
        matches!(payload, TypeToken::Tuple(_) | TypeToken::Struct { .. })
    })
}

fn contains_option(token: &TypeToken, payload: &impl Fn(&TypeToken) -> bool) -> bool {
    match token {
        TypeToken::Option(value) => payload(value),
        TypeToken::Tuple(values) => values.iter().any(|value| contains_option(value, payload)),
        TypeToken::Result { ok, err } => {
            contains_option(ok, payload) || contains_option(err, payload)
        }
        TypeToken::Array { element, .. }
        | TypeToken::Shared(element)
        | TypeToken::Slice(element) => contains_option(element, payload),
        TypeToken::Struct { fields, .. } => fields
            .iter()
            .any(|(_, field)| contains_option(field, payload)),
        _ => false,
    }
}

pub(crate) fn option_product(token: &TypeToken) -> bool {
    option_product_with(token, &|_| true)
}

/// Reference-free product shape is independent of nominal ownership authority.
/// The supplied check must prove each exact struct when ownership is required.
pub(crate) fn option_product_with(
    token: &TypeToken,
    nominal: &impl Fn(&TypeToken) -> bool,
) -> bool {
    match token {
        TypeToken::Scalar(value) => crate::concrete_types::scalar(value),
        TypeToken::Tuple(values) => values
            .iter()
            .all(|value| option_product_with(value, nominal)),
        TypeToken::Struct { fields, .. } => {
            nominal(token)
                && fields
                    .iter()
                    .all(|(_, field)| option_product_with(field, nominal))
        }
        _ => false,
    }
}

fn check(token: &TypeToken, depth: usize, entries: &mut usize) -> Result<(), String> {
    *entries += 1;
    if depth > crate::concrete_types::MAX_DEPTH || *entries > crate::concrete_types::MAX_ENTRIES {
        return Err("Data interface token exceeds recursive budget".into());
    }
    if let Some(element) = recursive_element(token) {
        return check(element, depth + 1, entries);
    }

    check_contents(token, depth, entries)
}

fn check_contents(token: &TypeToken, depth: usize, entries: &mut usize) -> Result<(), String> {
    match token {
        TypeToken::NonZero(observation) => observation.check_shape(),
        TypeToken::MutableScalar(value) if depth == 0 && value == "usize" => Ok(()),
        TypeToken::Scalar(value)
            if scalar(value) || depth == 0 && matches!(value.as_str(), "char" | "f32" | "f64") =>
        {
            Ok(())
        }
        TypeToken::Tuple(values) => check_tuple(values, depth, entries),
        TypeToken::Result { ok, err } => check_result(ok, err, depth, entries),
        TypeToken::Str => Ok(()),
        TypeToken::RangeUsize | TypeToken::RangeToUsize | TypeToken::RangeFromUsize
            if depth == 0 =>
        {
            Ok(())
        }
        TypeToken::FieldlessEnum { nominal, variants }
            if nominal
                .source_identity
                .as_ref()
                .is_some_and(|id| !id.is_empty())
                && !variants.is_empty()
                && variants.len() <= crate::concrete_types::MAX_FIELDS =>
        {
            check_fieldless_variants(variants)
        }
        TypeToken::IntConversionResult(value)
            if matches!(value.as_str(), "usize" | "u16" | "u64") =>
        {
            Ok(())
        }
        TypeToken::SliceArrayResult { length, .. }
            if *length <= crate::concrete_types::MAX_FIELDS as u64 =>
        {
            Ok(())
        }
        TypeToken::ScalarProductInstance { .. }
        | TypeToken::MixedEnum { .. }
        | TypeToken::Struct { .. }
        | TypeToken::Enum { .. } => check_nominal(token, depth, entries),
        // Admission additionally requires independently verified nominal source/effects.
        _ => Err("Data type token lacks a registered concrete proof".into()),
    }
}

/// Observational payload only; the opaque conversion error has no inferred layout.
pub(crate) fn mapped_int_conversion_payload(token: &TypeToken) -> bool {
    matches!(token, TypeToken::Tuple(values)
        if matches!(values.as_slice(), [TypeToken::Scalar(value), TypeToken::Scalar(count)]
            if value == "usize" && count == "usize"))
}

fn check_tuple(values: &[TypeToken], depth: usize, entries: &mut usize) -> Result<(), String> {
    for value in values {
        check(value, depth + 1, entries)?;
    }
    Ok(())
}

fn check_result(
    ok: &TypeToken,
    err: &TypeToken,
    depth: usize,
    entries: &mut usize,
) -> Result<(), String> {
    let unit_result = matches!(ok, TypeToken::Scalar(value) if value == "unit")
        && matches!(err, TypeToken::Struct { fields, .. } if matches!(fields.as_slice(), [(name, TypeToken::Scalar(value))] if name == "0" && value == "unit"));
    if !unit_result
        && !crate::concrete_types::payload_errors::GRAMMAR.permits(ok, err)
        && !crate::concrete_types::static_string_results::GRAMMAR.permits(ok, err)
        && !crate::result_maps::types::token_result(ok, err)
        && !crate::borrowed_result_types::GRAMMAR.permits(ok, err)
        && !crate::concrete_results::copy_errors::GRAMMAR.permits(ok, err)
        && !crate::concrete_results::fresh_products::GRAMMAR.permits(ok, err)
        && !crate::concrete_results::scalar_errors::GRAMMAR.permits(ok, err)
    {
        return Err(
            "Concrete Result token requires unit success and a unit tuple error struct".into(),
        );
    }
    check(ok, depth + 1, entries)?;
    check(err, depth + 1, entries)
}

fn check_nominal(token: &TypeToken, depth: usize, entries: &mut usize) -> Result<(), String> {
    match token {
        TypeToken::ScalarProductInstance { .. } => {
            scalar_product_instances::check(token, depth, entries)
        }
        TypeToken::MixedEnum { nominal, variants } => {
            mixed_owners::check(nominal, variants, depth, entries)
        }
        TypeToken::Struct { fields, .. }
        | TypeToken::Enum {
            variants: fields, ..
        } if valid_nominal(token) => check_nominal_fields(fields, depth, entries),
        _ => Err("Data type token lacks a registered concrete proof".into()),
    }
}

fn check_fieldless_variants(variants: &[(String, i64)]) -> Result<(), String> {
    let names: std::collections::BTreeSet<_> = variants.iter().map(|(name, _)| name).collect();
    let values: std::collections::BTreeSet<_> = variants.iter().map(|(_, value)| value).collect();
    if names.len() == variants.len()
        && values.len() == variants.len()
        && names.iter().all(|name| !name.is_empty())
    {
        Ok(())
    } else {
        Err("Fieldless enum repeats or lacks a variant".into())
    }
}

fn recursive_element(token: &TypeToken) -> Option<&TypeToken> {
    match token {
        TypeToken::MappedIntConversionResult { ok } if mapped_int_conversion_payload(ok) => {
            Some(ok)
        }
        TypeToken::Option(payload) if matches!(payload.as_ref(), TypeToken::NonZero(_)) => {
            Some(payload)
        }
        TypeToken::Option(payload)
            if matches!(payload.as_ref(), TypeToken::ScalarProductInstance { .. }) =>
        {
            Some(payload)
        }
        TypeToken::Option(payload) if shared_byte_slice(payload.as_ref()) => Some(payload),
        TypeToken::Array { element, .. }
        | TypeToken::Shared(element)
        | TypeToken::Slice(element) => Some(element),
        TypeToken::Option(payload) if matches!(payload.as_ref(), TypeToken::Shared(value) if matches!(value.as_ref(), TypeToken::Scalar(scalar) if scalar != "unit" && crate::concrete_types::scalar(scalar))) => {
            Some(payload)
        }
        TypeToken::Option(payload)
            if matches!(
                payload.as_ref(),
                TypeToken::Tuple(_) | TypeToken::Struct { .. }
            ) && option_product(payload) =>
        {
            Some(payload)
        }
        TypeToken::Option(payload) if matches!(payload.as_ref(), TypeToken::Scalar(scalar) if scalar != "unit" && crate::concrete_types::scalar(scalar)) => {
            Some(payload)
        }
        _ => None,
    }
}

fn check_nominal_fields(
    fields: &[(String, TypeToken)],
    depth: usize,
    entries: &mut usize,
) -> Result<(), String> {
    let mut names = std::collections::BTreeSet::new();
    for (name, field) in fields {
        if name.is_empty() || !names.insert(name) {
            return Err("Invalid nominal field vocabulary".into());
        }
        check(field, depth + 1, entries)?;
    }
    Ok(())
}

fn valid_nominal(token: &TypeToken) -> bool {
    let (nominal, valid) = match token {
        TypeToken::Struct { nominal, fields } => (nominal, fields.len() <= crate::concrete_types::MAX_FIELDS),
        TypeToken::Enum { nominal, variants } => (nominal, !variants.is_empty() && variants.len() <= 16
            && variants.iter().all(|(_, payload)| matches!(payload, TypeToken::Scalar(value) if scalar(value) && value != "unit"))),
        _ => return false,
    };
    valid
        && nominal
            .source_identity
            .as_ref()
            .is_some_and(|id| !id.is_empty())
}

fn check_conversion_capabilities(
    text: &str,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Result<(), String> {
    if text.contains("\"kind\":\"int_conversion_result\",\"value\":\"u64\"")
        && !profile.supports("rust.lossless-usize-u64-calls")
    {
        return Err("Lossless u64 Result requires its registered capability".into());
    }
    if text.contains("\"kind\":\"int_conversion_result\"")
        && !profile.supports("rust.bitset-cardinality")
    {
        return Err("Integer Result requires its registered capability".into());
    }
    if text.contains("\"kind\":\"int_conversion_result\",\"value\":\"u16\"")
        && !profile.supports("rust.end-of-input")
    {
        return Err("Narrowing Result requires its registered capability".into());
    }
    Ok(())
}
