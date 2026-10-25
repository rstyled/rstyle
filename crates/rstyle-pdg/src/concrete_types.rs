//! Bounded, observational concrete type evidence. Never authorizes admission.

pub(crate) mod byte_views;
mod declarations;
mod fieldless_enums;
pub use fieldless_enums::{
    EXPLICIT_U8_CAPABILITY, explicit_u8_fieldless_declaration, explicit_u8_fieldless_variants,
    explicit_u8_repr, explicit_u8_variant_expect, fieldless_variants,
};
pub mod lifetimes;
mod mixed_enums;
mod option_spelling;
pub(crate) use option_spelling::result_arguments;
mod scalar_enums;
pub(crate) mod static_string_results;
pub use static_string_results::observed_success as observed_static_string_result_success;
mod validation;
pub(crate) use mixed_enums::closed_payload;
pub use mixed_enums::mixed_variant_declarations;
pub use scalar_enums::variant_declarations;
pub mod source_proof;

use crate::source_evidence::byte_offset;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const KEY: &str = "concrete_type_observations";
pub const IDENTITY: &str = "rstyle.rust.concrete-type-capture/1";
pub const MAX_DEPTH: usize = 16;
pub const MAX_VARIANTS: usize = 16;
pub const MAX_ENTRIES: usize = 4096;
pub const MAX_FIELDS: usize = 128;
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "evidence", deny_unknown_fields)]
pub enum TypeTree {
    Scalar(String),
    /// Exact core nonzero integer identity; source/value permissions are separate.
    NonZero(crate::nonzero_values::Observation),
    Tuple(Vec<TypeTree>),
    Array {
        element: Box<TypeTree>,
        length: u64,
    },
    Shared(Box<TypeTree>),
    /// Root-only mutable scalar descriptor; memory permission is checked separately.
    MutableScalar(String),
    Slice(Box<TypeTree>),
    Str,
    Option(Box<TypeTree>),
    /// Observed core Result shape; contextual source/drop/effect proof is separate.
    Result {
        ok: Box<TypeTree>,
        err: Box<TypeTree>,
    },
    /// Core Result<T, TryFromIntError>; no error layout is inferred.
    IntConversionResult(String),
    /// Pinned non-Copy Range<usize>; fresh result use is checked separately.
    RangeUsize {
        observation: crate::range_values::authority::Observation,
    },
    /// Observed non-Copy checked suffix carrier; no standalone value permission.
    RangeFromUsize {
        observation: crate::range_values::authority::Observation,
    },
    RangeToUsize {
        observation: crate::range_values::authority::Observation,
    },
    SliceArrayResult {
        length: u64,
        borrowed: bool,
    },
    Struct {
        nominal: Nominal,
        source: crate::SourceText,
        fields: Vec<Field>,
    },
    /// Exact scalar substitution in an original generic tuple declaration.
    /// Shape capture grants no ownership, cleanup or value-use permission.
    ScalarProductInstance(Box<crate::scalar_product_instances::Observation>),
    /// Raw local expansion observation. The template is original input bytes,
    /// not emitted Rust source; generated ownership must be checked separately.
    GeneratedStruct {
        nominal: Nominal,
        template: crate::SourceText,
        definition: Box<crate::generated_sources::capture::Definition>,
        fields: Vec<Field>,
    },
    FieldlessEnum {
        nominal: Nominal,
        source: crate::SourceText,
        variants: Vec<(String, i64)>,
    },
    Enum {
        nominal: Nominal,
        source: crate::SourceText,
        variants: Vec<Variant>,
    },
    MixedEnum {
        nominal: Nominal,
        source: crate::SourceText,
        variants: Vec<MixedVariant>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub position: usize,
    pub name: String,
    pub payload: Field,
}

/// A mandatory tagged shape; missing payload evidence never denotes a unit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum VariantShape<T> {
    Unit {},
    SingleTuple { field: T },
    Named { fields: Vec<(String, T)> },
}

impl<T> VariantShape<T> {
    pub fn field(&self) -> Option<&T> {
        match self {
            Self::Unit {} | Self::Named { .. } => None,
            Self::SingleTuple { field } => Some(field),
        }
    }

    /// Every payload child, including named fields in declaration order.
    pub fn fields(&self) -> impl Iterator<Item = &T> {
        let single = match self {
            Self::SingleTuple { field } => Some(field),
            _ => None,
        };
        let named = match self {
            Self::Named { fields } => Some(fields),
            _ => None,
        };
        single
            .into_iter()
            .chain(named.into_iter().flatten().map(|(_, field)| field))
    }

    pub fn map<U>(&self, mut transform: impl FnMut(&T) -> U) -> VariantShape<U> {
        match self {
            Self::Unit {} => VariantShape::Unit {},
            Self::SingleTuple { field } => VariantShape::SingleTuple {
                field: transform(field),
            },
            Self::Named { fields } => VariantShape::Named {
                fields: fields
                    .iter()
                    .map(|(name, field)| (name.clone(), transform(field)))
                    .collect(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MixedVariant {
    pub position: usize,
    pub name: String,
    pub shape: VariantShape<Field>,
}

impl TypeTree {
    pub(crate) fn wrapped(self, wrapper: impl FnOnce(Box<Self>) -> Self) -> Self {
        wrapper(Box::new(self))
    }

    pub(crate) fn nominal_parts(&self) -> Option<(&Nominal, &crate::SourceText)> {
        match self {
            Self::ScalarProductInstance(instance) => Some((&instance.nominal, &instance.source)),
            Self::Struct {
                nominal, source, ..
            }
            | Self::FieldlessEnum {
                nominal, source, ..
            }
            | Self::Enum {
                nominal, source, ..
            }
            | Self::MixedEnum {
                nominal, source, ..
            } => Some((nominal, source)),
            _ => None,
        }
    }

    pub(crate) fn generated_parts(
        &self,
    ) -> Option<(
        &Nominal,
        &crate::generated_sources::capture::Definition,
        &crate::SourceText,
        &[Field],
    )> {
        match self {
            Self::GeneratedStruct {
                nominal,
                definition,
                template,
                fields,
            } => Some((nominal, definition, template, fields)),
            _ => None,
        }
    }

    pub(crate) fn nominal_identity(&self) -> Option<&Nominal> {
        self.nominal_parts()
            .map(|(nominal, _)| nominal)
            .or_else(|| self.generated_parts().map(|(nominal, ..)| nominal))
    }

    /// Ordered children of the two supported multi-component container forms.
    pub(crate) fn components(&self) -> impl Iterator<Item = &Self> {
        let tuple = match self {
            Self::Tuple(values) => values.as_slice(),
            _ => &[],
        };
        let result = match self {
            Self::Result { ok, err } => Some((ok.as_ref(), err.as_ref())),
            _ => None,
        };
        tuple
            .iter()
            .chain(result.into_iter().flat_map(|(ok, err)| [ok, err]))
    }

    pub(crate) fn nominal_fields(&self) -> impl Iterator<Item = &Field> {
        let fields = match self {
            Self::ScalarProductInstance(instance) => Some(&instance.fields),
            Self::Struct { fields, .. } | Self::GeneratedStruct { fields, .. } => Some(fields),
            _ => None,
        };
        let variants = match self {
            Self::Enum { variants, .. } => Some(variants),
            _ => None,
        };
        let mixed = match self {
            Self::MixedEnum { variants, .. } => Some(variants),
            _ => None,
        };
        fields
            .into_iter()
            .flatten()
            .chain(
                variants
                    .into_iter()
                    .flatten()
                    .map(|variant| &variant.payload),
            )
            .chain(
                mixed
                    .into_iter()
                    .flatten()
                    .flat_map(|variant| variant.shape.fields()),
            )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Nominal {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_identity: Option<String>,
    pub package_name: String,
    pub package_version: String,
    pub manifest_digest: String,
    pub definition_path: String,
    pub declaration_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub position: usize,
    pub name: String,
    pub declaration: crate::SourceText,
    pub ty: TypeTree,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", deny_unknown_fields)]
pub enum Observation {
    Captured { ty: TypeTree },
    Unsupported { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterfaceObservations {
    pub identity: String,
    pub parameters: Vec<Observation>,
    pub result: Observation,
}

impl InterfaceObservations {
    pub fn validate(&self) -> Result<(), String> {
        if self.identity != IDENTITY {
            return Err("Unregistered concrete type observation identity".into());
        }
        validation::observations(self.parameters.iter().chain([&self.result]))
    }
}

pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn scalar(token: &str) -> bool {
    matches!(
        token,
        "bool"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "unit"
    )
}

pub fn validate(tree: &TypeTree) -> Result<(), String> {
    validation::tree(tree)
}

pub(crate) fn validate_observations<'a>(
    observations: impl Iterator<Item = &'a Observation>,
) -> Result<(), String> {
    validation::observations(observations)
}

/// Shape consistency for raw observations; this grants no source or semantic authority.
pub trait ObservationShape {
    fn check_shape(&self) -> Result<(), String>;
}

impl ObservationShape for crate::nonzero_values::Observation {
    fn check_shape(&self) -> Result<(), String> {
        self.check_shape()
    }
}

impl ObservationShape for crate::scalar_product_instances::Observation {
    fn check_shape(&self) -> Result<(), String> {
        self.check_shape()
    }
}

fn check_shape(tree: &TypeTree) -> Result<(), String> {
    match tree {
        TypeTree::NonZero(observation) => ObservationShape::check_shape(observation),
        TypeTree::ScalarProductInstance(instance) => ObservationShape::check_shape(instance.as_ref()),
        TypeTree::RangeUsize { observation } => crate::range_values::authority::shape(observation),
        TypeTree::RangeFromUsize { observation } => crate::range_values::authority::range_from_shape(observation),
        TypeTree::RangeToUsize { observation } => crate::range_values::authority::range_to_shape(observation),
        TypeTree::Result { ok, err } => result_shape(ok, err),
        TypeTree::IntConversionResult(value) if !matches!(value.as_str(), "usize" | "u16" | "u64") => {
            Err("Integer conversion Result requires a registered scalar payload".into())
        }
        TypeTree::Option(payload) if !option_payload(payload) => {
            Err("Option observation requires a registered scalar, shared scalar, or reference-free product payload".into())
        }
        TypeTree::SliceArrayResult { length, .. } if *length > MAX_FIELDS as u64 => {
            Err("Slice conversion result exceeds supported array length".into())
        }
        TypeTree::MutableScalar(token) if token == "usize" => Ok(()),
        TypeTree::MutableScalar(_) => Err("Mutable scalar observations support only usize".into()),
        TypeTree::Scalar(token) if scalar(token) => Ok(()),
        TypeTree::Scalar(_) => Err("Unsupported scalar in concrete type observation".into()),
        TypeTree::Struct {
            nominal,
            source,
            fields,
        } => check_declaration(nominal, source, fields),
        TypeTree::GeneratedStruct { .. } => crate::declaration_origins::generated::shape(tree),
        TypeTree::Enum {
            nominal,
            source,
            variants,
        } => scalar_enums::check(nominal, source, variants),
        TypeTree::MixedEnum {
            nominal,
            source,
            variants,
        } => mixed_enums::check(nominal, source, variants),
        TypeTree::FieldlessEnum {
            nominal,
            source,
            variants,
        } => fieldless_enums::check_with_explicit_u8(nominal, source, variants),
        _ => Ok(()),
    }
}

/// This bounded shape grants neither Copy, drop freedom nor value permission.
fn result_shape(ok: &TypeTree, err: &TypeTree) -> Result<(), String> {
    if static_string_results::observed(ok, err)
        || crate::result_maps::types::observed_result(ok, err)
        || crate::borrowed_result_types::tree_result(ok, err)
        || crate::concrete_results::fresh_products::GRAMMAR.permits(
            &crate::data_types::TypeToken::from_tree(ok),
            &crate::data_types::TypeToken::from_tree(err),
        )
        || crate::concrete_results::copy_errors::GRAMMAR.permits(
            &crate::data_types::TypeToken::from_tree(ok),
            &crate::data_types::TypeToken::from_tree(err),
        )
        || crate::concrete_results::scalar_errors::GRAMMAR.permits(
            &crate::data_types::TypeToken::from_tree(ok),
            &crate::data_types::TypeToken::from_tree(err),
        )
    {
        return Ok(());
    }
    let TypeTree::Struct {
        nominal,
        source,
        fields,
    } = err
    else {
        return Err("Concrete Result requires a complete local error struct".into());
    };
    if !matches!(ok, TypeTree::Scalar(value) if value == "unit")
        || !matches!(fields.as_slice(), [Field { position: 0, name, ty: TypeTree::Scalar(value), .. }] if name == "0" && value == "unit")
        || !nominal
            .source_identity
            .as_ref()
            .is_some_and(|identity| !identity.is_empty())
    {
        return Err(
            "Concrete Result requires unit success and a single unit tuple error field".into(),
        );
    }

    check_declaration(nominal, source, fields)?;
    let syntax: syn::ItemStruct = syn::parse_str(source).map_err(|error| error.to_string())?;
    if !syntax.generics.params.is_empty()
        || syntax.generics.where_clause.is_some()
        || !matches!(syntax.fields, syn::Fields::Unnamed(_))
    {
        return Err("Concrete Result error requires a nongeneric tuple struct declaration".into());
    }

    Ok(())
}

fn option_payload(tree: &TypeTree) -> bool {
    match tree {
        TypeTree::NonZero(_) => true,
        TypeTree::ScalarProductInstance(_) => true,
        TypeTree::Scalar(value) => scalar(value) && value != "unit",
        TypeTree::Shared(value) => {
            matches!(value.as_ref(), TypeTree::Scalar(value) if scalar(value) && value != "unit")
                || matches!(value.as_ref(), TypeTree::Slice(element) if matches!(element.as_ref(), TypeTree::Scalar(value) if value == "u8"))
        }
        TypeTree::Tuple(_) | TypeTree::Struct { .. } => {
            crate::data_types::option_product(&crate::data_types::TypeToken::from_tree(tree))
        }
        _ => false,
    }
}

pub(crate) fn check_declaration_budget(source: &str, fields: &[Field]) -> Result<(), String> {
    if source.len() > MAX_SOURCE_BYTES || fields.len() > MAX_FIELDS {
        return Err("Concrete declaration exceeds source/field budget".into());
    }
    Ok(())
}

fn check_declaration(
    nominal: &Nominal,
    source: &crate::SourceText,
    fields: &[Field],
) -> Result<(), String> {
    check_declaration_budget(source, fields)?;
    check_identity(nominal, source)?;
    let declaration: syn::ItemStruct = syn::parse_str(source)
        .map_err(|error| format!("Invalid concrete struct declaration: {error}"))?;
    if nominal.definition_path.rsplit("::").next() != Some(declaration.ident.to_string().as_str())
        || !declaration.generics.params.is_empty()
            && lifetimes::parameter_count(&declaration.generics).is_err()
        || declaration.fields.len() != fields.len()
    {
        return Err("Concrete struct name, generics or field inventory disagrees".into());
    }
    for (position, (declared, captured)) in declaration.fields.iter().zip(fields).enumerate() {
        check_field(source, position, declared, captured)?;
    }

    Ok(())
}

fn check_identity(nominal: &Nominal, source: &crate::SourceText) -> Result<(), String> {
    if nominal.package_name.is_empty()
        || nominal.package_version.is_empty()
        || !is_digest(&nominal.manifest_digest)
        || !source.matches_identity(&nominal.declaration_digest)
    {
        return Err(
            "Concrete nominal source/package identity is incomplete or inconsistent".into(),
        );
    }
    Ok(())
}

fn check_field(
    source: &str,
    position: usize,
    declared: &syn::Field,
    captured: &Field,
) -> Result<(), String> {
    use syn::spanned::Spanned;
    let name = declared
        .ident
        .as_ref()
        .map_or_else(|| position.to_string(), ToString::to_string);
    let span = declared.ty.span();
    let start = byte_offset(source, span.start());
    let end = byte_offset(source, span.end());
    if captured.position != position
        || captured.name != name
        || source.get(start..end) != Some(captured.declaration.as_str())
    {
        return Err(
            "Concrete field identity/order/declaration disagrees with original source".into(),
        );
    }

    declarations::check(&declared.ty, &captured.ty)?;

    Ok(())
}

pub fn field_declarations(source: &str) -> Result<Vec<(String, String)>, String> {
    use syn::spanned::Spanned;
    let declaration: syn::ItemStruct = syn::parse_str(source).map_err(|error| error.to_string())?;
    declaration
        .fields
        .iter()
        .enumerate()
        .map(|(position, field)| {
            let name = field
                .ident
                .as_ref()
                .map_or_else(|| position.to_string(), ToString::to_string);
            let span = field.ty.span();
            let spelling = source
                .get(byte_offset(source, span.start())..byte_offset(source, span.end()))
                .ok_or("Missing concrete field source bytes")?
                .to_owned();
            Ok((name, spelling))
        })
        .collect()
}

fn is_digest(value: &str) -> bool {
    crate::source_identity::Scheme::parse(value).is_some()
}

#[cfg(test)]
mod mutable_scalar_tests;
