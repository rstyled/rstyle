//! Bounded observations of original generic byte-view/scalar tuple products.
//! Shape consistency grants no source ownership, Copy, cleanup or value authority.
pub(crate) mod activation;
pub mod constructors;
pub(crate) mod forwarders;
pub(crate) mod origins;
mod payload;
pub mod transformation;

use crate::concrete_types::byte_views::{
    explicit_lifetime_source as source_view, observed as observed_view,
};
use crate::concrete_types::{self, Field, Nominal, TypeTree};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Argument {
    U8,
    U16,
    #[serde(rename = "nonzero_u8")]
    NonZeroU8,
    #[serde(rename = "nonzero_u16")]
    NonZeroU16,
}

impl Argument {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::NonZeroU8 => "NonZero<u8>",
            Self::NonZeroU16 => "NonZero<u16>",
        }
    }

    pub fn matches_tree(self, tree: &TypeTree) -> bool {
        self.matches_payload(tree)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub nominal: Nominal,
    pub source: crate::SourceText,
    pub argument: Argument,
    pub fields: Vec<Field>,
}

impl Observation {
    /// Check original declaration bytes and exact bounded scalar substitution.
    /// Compiler instance selection and contextual source closure remain separate.
    pub fn check_shape(&self) -> Result<(), String> {
        concrete_types::check_declaration_budget(&self.source, &self.fields)?;
        self.check_identity()?;
        let declaration: syn::ItemStruct = syn::parse_str(&self.source)
            .map_err(|error| format!("Invalid scalar product declaration: {error}"))?;
        if self.nominal.definition_path.rsplit("::").next()
            != Some(declaration.ident.to_string().as_str())
        {
            return Err("Scalar product original declaration name disagrees".into());
        }
        let (lifetime, parameter) = parameters(&declaration.generics)?;

        self.check_fields(&declaration, lifetime, parameter)
    }

    fn check_identity(&self) -> Result<(), String> {
        if self.nominal.package_name.is_empty()
            || self.nominal.package_version.is_empty()
            || crate::source_identity::Scheme::parse(&self.nominal.manifest_digest).is_none()
            || !self
                .source
                .matches_identity(&self.nominal.declaration_digest)
        {
            return Err("Scalar product source/package identity is inconsistent".into());
        }

        Ok(())
    }

    fn check_fields(
        &self,
        declaration: &syn::ItemStruct,
        lifetime: &syn::Lifetime,
        parameter: &syn::Ident,
    ) -> Result<(), String> {
        let syn::Fields::Unnamed(source_fields) = &declaration.fields else {
            return Err("Scalar product requires an original tuple declaration".into());
        };
        let [view, scalar] = self.fields.as_slice() else {
            return Err("Scalar product requires exactly two observed fields".into());
        };
        if source_fields.unnamed.len() != 2
            || source_fields
                .unnamed
                .iter()
                .any(|field| !field.attrs.is_empty())
        {
            return Err("Scalar product requires two unconditional source fields".into());
        }
        self.check_spellings()?;

        if !source_view(&source_fields.unnamed[0].ty, lifetime)
            || !matches!(&source_fields.unnamed[1].ty, syn::Type::Path(path)
                if path.qself.is_none() && path.path.is_ident(parameter))
            || !observed_view(&view.ty)
            || !self.argument.matches_tree(&scalar.ty)
        {
            return Err("Scalar product original fields or concrete substitution disagree".into());
        }

        Ok(())
    }

    fn check_spellings(&self) -> Result<(), String> {
        for (position, (field, (name, spelling))) in self
            .fields
            .iter()
            .zip(concrete_types::field_declarations(&self.source)?)
            .enumerate()
        {
            if field.position != position || field.name != name || field.declaration != spelling {
                return Err("Scalar product field order or original spelling disagrees".into());
            }
        }

        Ok(())
    }
}

fn parameters(generics: &syn::Generics) -> Result<(&syn::Lifetime, &syn::Ident), String> {
    let mut parameters = generics.params.iter();
    let (Some(syn::GenericParam::Lifetime(lifetime)), Some(syn::GenericParam::Type(scalar))) =
        (parameters.next(), parameters.next())
    else {
        return Err("Scalar product requires one lifetime then one type parameter".into());
    };
    if parameters.next().is_some()
        || generics.where_clause.is_some()
        || !lifetime.attrs.is_empty()
        || !lifetime.bounds.is_empty()
        || matches!(lifetime.lifetime.ident.to_string().as_str(), "static" | "_")
        || !scalar.attrs.is_empty()
        || !scalar.bounds.is_empty()
        || scalar.default.is_some()
        || scalar.ident == "u8"
    {
        return Err("Scalar product requires unbounded original lifetime/type parameters".into());
    }

    Ok((&lifetime.lifetime, &scalar.ident))
}

#[cfg(test)]
mod tests;
