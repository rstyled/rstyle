//! Compact integer range tests over compile-time literal and constant bounds.

use crate::source_evidence::byte_offset;
use crate::{ByteSpan, HirNode, scalar_constants::ScalarConstant};
use serde::{Deserialize, Serialize};
use syn::{parse::Parser, spanned::Spanned};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegerRange {
    pub start: Option<String>,
    pub end: Option<String>,
    pub inclusive: bool,
}

struct RangePolicy {
    width: Option<u64>,
    constants: bool,
    bytes: bool,
}

impl IntegerRange {
    pub fn text(&self) -> String {
        serde_json::to_string(self).expect("integer range strings")
    }

    pub fn from_source(source: &str) -> Result<Self, String> {
        let pattern = syn::Pat::parse_single
            .parse_str(source)
            .map_err(|e| e.to_string())?;
        let syn::Pat::Range(range) = pattern else {
            return Err("Expected an integer range pattern".into());
        };
        if !integer_range(&range) {
            return Err("Range bounds must be integer literals".into());
        }

        Ok(Self {
            start: range.start.as_ref().map(|value| spelling(source, value)),
            end: range.end.as_ref().map(|value| spelling(source, value)),
            inclusive: matches!(range.limits, syn::RangeLimits::Closed(_)),
        })
    }

    pub fn from_pattern(
        source: &str,
        pattern: &HirNode,
        width: Option<u64>,
        constants: bool,
    ) -> Result<Self, String> {
        Self::resolve_pattern(
            source,
            pattern,
            RangePolicy {
                width,
                constants,
                bytes: false,
            },
        )
    }

    pub fn from_byte_pattern(
        source: &str,
        pattern: &HirNode,
        width: Option<u64>,
        constants: bool,
    ) -> Result<Self, String> {
        Self::resolve_pattern(
            source,
            pattern,
            RangePolicy {
                width,
                constants,
                bytes: true,
            },
        )
    }

    fn resolve_pattern(
        source: &str,
        pattern: &HirNode,
        policy: RangePolicy,
    ) -> Result<Self, String> {
        let mut syntax = syn::Pat::parse_single
            .parse_str(source)
            .map_err(|error| error.to_string())?;
        while let syn::Pat::Paren(pattern) = syntax {
            syntax = *pattern.pat;
        }
        let syn::Pat::Range(range) = syntax else {
            return Err("Expected an integer range pattern".into());
        };
        if !scalar_range_for(&range, policy.constants, policy.bytes) {
            return Err("Unsupported integer range bounds".into());
        }
        let records = constant_bounds(pattern, policy.width)?;
        let paths = range
            .start
            .iter()
            .chain(&range.end)
            .filter(|value| matches!(&***value, syn::Expr::Path(_)))
            .count();
        if records.len() != paths {
            return Err("Range bound correspondence disagrees".into());
        }

        Ok(Self {
            start: range
                .start
                .as_deref()
                .map(|value| resolved_bound(source, value, pattern.span.start, &records))
                .transpose()?,
            end: range
                .end
                .as_deref()
                .map(|value| resolved_bound(source, value, pattern.span.start, &records))
                .transpose()?,
            inclusive: matches!(range.limits, syn::RangeLimits::Closed(_)),
        })
    }
}

pub fn integer_range(range: &syn::PatRange) -> bool {
    scalar_range(range, false)
}

pub fn scalar_range(range: &syn::PatRange, constants: bool) -> bool {
    scalar_range_for(range, constants, false)
}

pub fn scalar_range_for(range: &syn::PatRange, constants: bool, bytes: bool) -> bool {
    (range.start.is_some() || range.end.is_some())
        && (!matches!(range.limits, syn::RangeLimits::Closed(_)) || range.end.is_some())
        && range.start.iter().chain(&range.end).all(|value| {
            integer_bound(value)
                || (bytes && matches!(&**value, syn::Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Byte(_))))
                || (constants && matches!(&**value, syn::Expr::Path(_)))
        })
}

fn constant_bounds(
    pattern: &HirNode,
    width: Option<u64>,
) -> Result<Vec<(ByteSpan, String)>, String> {
    pattern
        .children
        .iter()
        .map(|bound| {
            let constant = ScalarConstant::verify_pattern(bound, width)?;
            if bound.ty != pattern.ty {
                return Err("Constant range bound type disagrees".into());
            }
            Ok((bound.span, constant.value))
        })
        .collect()
}

fn resolved_bound(
    source: &str,
    value: &syn::Expr,
    offset: u32,
    records: &[(ByteSpan, String)],
) -> Result<String, String> {
    if integer_bound(value)
        || matches!(value, syn::Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Byte(_)))
    {
        return Ok(spelling(source, value));
    }
    let span = value.span();
    let expected = ByteSpan {
        start: offset + byte_offset(source, span.start()) as u32,
        end: offset + byte_offset(source, span.end()) as u32,
    };

    records
        .iter()
        .find(|(span, _)| span == &expected)
        .map(|(_, value)| value.clone())
        .ok_or_else(|| "Missing original constant range bound correspondence".into())
}

fn integer_bound(value: &syn::Expr) -> bool {
    match value {
        syn::Expr::Lit(literal) => matches!(literal.lit, syn::Lit::Int(_)),
        syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
            matches!(&*unary.expr, syn::Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Int(_)))
        }
        _ => false,
    }
}

fn spelling(source: &str, value: &syn::Expr) -> String {
    let span = value.span();
    source[byte_offset(source, span.start())..byte_offset(source, span.end())].into()
}
