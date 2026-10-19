//! Standard, unformatted assertion invocations and their source correspondence.

use crate::{ByteSpan, HirNode, source_evidence::byte_offset};
use syn::{parse::Parser, punctuated::Punctuated, spanned::Spanned};

pub struct Assertion {
    pub name: &'static str,
    pub condition: syn::Expr,
}

pub fn macro_name(path: &str) -> Option<&'static str> {
    match path.trim_start_matches("::") {
        "assert" | "core::assert" | "std::assert" => Some("assert"),
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
        if arguments.len() != 1 {
            return Err("Assertion formatting arguments are deferred".into());
        }

        Ok(Self {
            name,
            condition: arguments.into_iter().next().expect("one argument"),
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

pub fn verify(node: &HirNode, source: &str, debug: Option<bool>) -> Result<(), String> {
    let assertion = Assertion::parse(source)?;
    let [condition] = node.children.as_slice() else {
        return Err("Assertion requires exactly one condition".into());
    };
    let active = if assertion.name == "debug_assert" {
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
        || normalized(condition) != Some("bool")
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

fn normalized(node: &HirNode) -> Option<&str> {
    node.ty.as_ref()?.normalized.as_deref()
}
