//! Exhaustive bounded repetition parsing; values are never callable summaries.
use super::*;
use proc_macro2::TokenStream;
use std::collections::BTreeSet;
use syn::{
    Token, braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
};

pub(crate) struct Unit {
    name: String,
    source: FileSpan,
    ratios: Vec<Ratio>,
}

impl Unit {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
    pub(crate) fn source(&self) -> &FileSpan {
        &self.source
    }
    pub(crate) fn ratios(&self) -> &[Ratio] {
        &self.ratios
    }
}

pub(crate) struct Ratio {
    larger: String,
    default_output: String,
    integers: Vec<String>,
    floats: Vec<String>,
}

impl Ratio {
    pub(crate) fn larger(&self) -> &str {
        &self.larger
    }
    pub(crate) fn default_output(&self) -> &str {
        &self.default_output
    }
    pub(crate) fn integers(&self) -> &[String] {
        &self.integers
    }
    pub(crate) fn floats(&self) -> &[String] {
        &self.floats
    }
}

pub(super) struct Invocation(Vec<RawUnit>);
struct RawUnit {
    name: syn::Ident,
    ratios: Vec<Ratio>,
}

impl Invocation {
    pub(super) fn parse(tokens: TokenStream, file: &UnitFile) -> Result<Vec<Unit>, ProofError> {
        let parsed: Self = syn::parse2(tokens).map_err(|_| {
            ProofError::Unsupported(
                "Generated-unit invocation is outside the complete bounded grammar",
            )
        })?;
        parsed.validate()?;
        Ok(parsed
            .0
            .into_iter()
            .map(|unit| Unit {
                source: site(file, unit.name.span()),
                name: unit.name.to_string(),
                ratios: unit.ratios,
            })
            .collect())
    }

    fn validate(&self) -> Result<(), ProofError> {
        let names: Vec<_> = self.0.iter().map(|unit| unit.name.to_string()).collect();
        if names.len() != 8
            || names.iter().collect::<BTreeSet<_>>().len() != 8
            || names.iter().any(|name| forbidden(name))
        {
            return Err(ProofError::Unsupported(
                "Generated units require eight distinct nonshadowing declarations",
            ));
        }
        for (ordinal, unit) in self.0.iter().enumerate() {
            let expected = &names[ordinal..];
            let actual: Vec<_> = unit.ratios.iter().map(|ratio| &ratio.larger).collect();
            if actual != expected.iter().collect::<Vec<_>>() {
                return Err(ProofError::Contradictory(
                    "Generated-unit ratio roster has missing, duplicate or foreign owners",
                ));
            }
        }
        Ok(())
    }
}

impl Parse for Invocation {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut units = Vec::new();
        while !input.is_empty() {
            if units.len() == 8 {
                return Err(input.error("Generated-unit declaration budget exceeded"));
            }
            let name = input.parse()?;
            let label;
            parenthesized!(label in input);
            label.parse::<syn::LitStr>()?;
            consumed(&label)?;
            let per: syn::Ident = input.parse()?;
            if per != "per" {
                return Err(input.error("Expected original per repetition separator"));
            }
            let body;
            braced!(body in input);
            let mut ratios = Vec::new();
            while !body.is_empty() {
                if ratios.len() == 8 {
                    return Err(body.error("Generated-unit ratio budget exceeded"));
                }
                ratios.push(body.parse()?);
            }
            units.push(RawUnit { name, ratios });
        }
        Ok(Self(units))
    }
}

impl Parse for Ratio {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let larger = input.parse::<syn::Ident>()?.to_string();
        input.parse::<Token![:]>()?;
        let default;
        bracketed!(default in input);
        let default_output = default.parse::<syn::Ident>()?.to_string();
        consumed(&default)?;
        let integers = types(input, false)?;
        integer_value(input)?;
        let floats = types(input, true)?;
        float_value(input)?;
        if !integers.contains(&default_output) {
            return Err(input.error("Default output is not an explicit integer implementation"));
        }
        Ok(Self {
            larger,
            default_output,
            integers,
            floats,
        })
    }
}

fn types(input: ParseStream<'_>, float: bool) -> syn::Result<Vec<String>> {
    let mut types = Vec::new();
    loop {
        let ident = input.parse::<syn::Ident>()?;
        let name = ident.to_string();
        if numeric(&name) != Some(float) || types.contains(&name) || types.len() == 12 {
            return Err(syn::Error::new(
                ident.span(),
                "Expected distinct original primitive outputs",
            ));
        }
        types.push(name);
        if !input.peek(Token![|]) {
            input.parse::<Token![=]>()?;
            return Ok(types);
        }
        input.parse::<Token![|]>()?;
    }
}

fn integer_value(input: ParseStream<'_>) -> syn::Result<()> {
    let value: syn::LitInt = input.parse()?;
    if !value.suffix().is_empty() || value.base10_parse::<u128>().is_err() {
        return Err(syn::Error::new(
            value.span(),
            "Expected a bounded unsuffixed integer literal",
        ));
    }
    input.parse::<Token![;]>()?;
    Ok(())
}

fn float_value(input: ParseStream<'_>) -> syn::Result<()> {
    let value: syn::LitFloat = input.parse()?;
    if !value.suffix().is_empty() || !value.base10_parse::<f64>().is_ok_and(f64::is_finite) {
        return Err(syn::Error::new(
            value.span(),
            "Expected a finite unsuffixed float literal",
        ));
    }
    input.parse::<Token![;]>()?;
    Ok(())
}

fn consumed(input: ParseStream<'_>) -> syn::Result<()> {
    if !input.is_empty() {
        return Err(input.error("Unexpected trailing original invocation tokens"));
    }
    Ok(())
}

fn numeric(name: &str) -> Option<bool> {
    match name {
        "f32" | "f64" => Some(true),
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128"
        | "isize" => Some(false),
        _ => None,
    }
}

fn forbidden(name: &str) -> bool {
    numeric(name).is_some()
        || matches!(
            name,
            "bool"
                | "char"
                | "str"
                | "Drop"
                | "Debug"
                | "Clone"
                | "Copy"
                | "MultipleOf"
                | "DefaultOutput"
                | "sealed"
                | "multiple_of_value"
                | "stringify_outputs"
                | "concat"
                | "stringify"
        )
}
