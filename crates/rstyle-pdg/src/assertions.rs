//! Standard assertion invocations with independently checked source correspondence.
mod comparison;
pub mod formatting;

use crate::frontend::normalized;
use crate::{ByteSpan, HirNode, source_evidence::byte_offset};
use syn::{parse::Parser, punctuated::Punctuated, spanned::Spanned};

pub struct Assertion {
    pub name: &'static str,
    pub condition: syn::Expr,
    pub right: Option<syn::Expr>,
    pub diagnostic: Option<formatting::Diagnostic>,
}

pub fn macro_name(path: &str) -> Option<&'static str> {
    match path.trim_start_matches("::") {
        "assert" | "core::assert" | "std::assert" => Some("assert"),
        "assert_eq" | "core::assert_eq" | "std::assert_eq" => Some("assert_eq"),
        "debug_assert_eq" | "core::debug_assert_eq" | "std::debug_assert_eq" => {
            Some("debug_assert_eq")
        }
        "debug_assert" | "core::debug_assert" | "std::debug_assert" => Some("debug_assert"),
        _ => None,
    }
}

impl Assertion {
    pub fn parse(source: &str) -> Result<Self, String> {
        let expression: syn::ExprMacro =
            syn::parse_str(source).map_err(|error| error.to_string())?;
        Self::from_macro(&expression.mac)
    }

    pub fn from_macro(mac: &syn::Macro) -> Result<Self, String> {
        let path = mac
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let name = macro_name(&path).ok_or("Not a registered assertion invocation")?;
        let arguments = Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
            .parse2(mac.tokens.clone())
            .map_err(|error| error.to_string())?;
        let comparison = name.ends_with("_eq");
        if arguments.is_empty() || arguments.len() > 2 || comparison && arguments.len() != 2 {
            return Err("Assertion formatting arguments are deferred".into());
        }

        let mut arguments = arguments.into_iter();
        let condition = arguments.next().expect("condition");
        let right = comparison.then(|| arguments.next().expect("right operand"));
        let diagnostic = arguments
            .next()
            .map(|argument| formatting::parse(&argument))
            .transpose()?;
        Ok(Self {
            name,
            condition,
            right,
            diagnostic,
        })
    }

    pub fn condition_span(&self, source: &str, offset: u32) -> ByteSpan {
        let span = self.condition.span();
        ByteSpan {
            start: offset + byte_offset(source, span.start()) as u32,
            end: offset + byte_offset(source, span.end()) as u32,
        }
    }
}

pub fn verify(
    node: &HirNode,
    source: &str,
    debug: Option<bool>,
    profile: crate::extraction_profile::ExtractionProfile,
) -> Result<(), String> {
    let assertion = Assertion::parse(source)?;
    let condition = node.children.first().ok_or("Missing assertion condition")?;
    if assertion.right.is_some() {
        if !profile.supports("rust.accelerator-packing") {
            return Err("Scalar comparison assertions require their registered capability".into());
        }
        comparison::verify(node, &assertion, source)?;
    } else if let Some(diagnostic) = &assertion.diagnostic {
        if !profile.supports("rust.end-of-input") {
            return Err("Formatted assertion requires its registered capability".into());
        }
        formatting::verify(node, source, diagnostic)?;
    } else if node.children.len() != 1 {
        return Err("Unformatted assertion requires exactly one condition".into());
    }
    let active = if assertion.name.starts_with("debug_") {
        debug.ok_or("Missing debug-assertions setting")?
    } else {
        true
    };
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing standard macro resolution")?;

    if node.kind != "expr_assert"
        || normalized(node) != Some("unit")
        || assertion.right.is_none() && normalized(condition) != Some("bool")
        || resolution.kind != "standard_assertion"
        || resolution.target.is_some()
        || resolution.description != format!("core::{}", assertion.name)
        || node.lit.as_deref() != Some(active.to_string().as_str())
        || assertion.condition_span(source, node.span.start) != condition.span
        || !node
            .expansion
            .as_ref()
            .is_some_and(|site| site.callsite == node.span && site.snippet == source)
    {
        return Err("Assertion source, resolution, condition or build setting disagrees".into());
    }
    Ok(())
}
