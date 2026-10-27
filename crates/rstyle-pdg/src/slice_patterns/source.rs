//! Independent original pattern proof; supplies no CFG, bounds or origin authority.
mod bindings;
mod nodes;
mod patterns;
mod scope;
#[cfg(test)]
mod tests;

use super::*;
use crate::{HirNode, unit_store::EvidenceFacts};
use syn::{parse::Parser, spanned::Spanned};

pub struct CheckedPrefix<'a> {
    pattern: &'a HirNode,
    digits: Vec<CheckedDigit<'a>>,
    remainder: CheckedRemainder<'a>,
}

pub struct CheckedDigit<'a> {
    binding: &'a HirNode,
    range: &'a HirNode,
    lower: &'a HirNode,
    upper: &'a HirNode,
}

pub struct CheckedRemainder<'a> {
    binding: &'a HirNode,
    rest: &'a HirNode,
}

impl<'a> CheckedPrefix<'a> {
    pub fn pattern(&self) -> &'a HirNode {
        self.pattern
    }
    pub fn digits(&self) -> &[CheckedDigit<'a>] {
        &self.digits
    }
    pub fn remainder(&self) -> &CheckedRemainder<'a> {
        &self.remainder
    }
}

impl<'a> CheckedDigit<'a> {
    pub fn binding(&self) -> &'a HirNode {
        self.binding
    }
    pub fn range(&self) -> &'a HirNode {
        self.range
    }
    pub fn lower(&self) -> &'a HirNode {
        self.lower
    }
    pub fn upper(&self) -> &'a HirNode {
        self.upper
    }
}

impl<'a> CheckedRemainder<'a> {
    pub fn binding(&self) -> &'a HirNode {
        self.binding
    }
    pub fn rest(&self) -> &'a HirNode {
        self.rest
    }
}

struct Proof<'a, 'f> {
    root: &'a HirNode,
    facts: &'f dyn EvidenceFacts,
    scope: String,
}

pub fn verify<'a>(
    root: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<CheckedPrefix<'a>, String> {
    let bytes = crate::source_evidence::byte_range(&facts.source.bytes, root.span)?;
    let syntax = syn::Pat::parse_single
        .parse_str(bytes)
        .map_err(|error| format!("Unsupported original slice pattern: {error}"))?;
    let syn::Pat::Slice(pattern) = syntax else {
        return Err(unsupported("Original pattern is not a slice"));
    };

    let proof = Proof {
        root,
        facts,
        scope: scope::verify(root, facts)?,
    };

    proof.prefix(&pattern)
}

impl<'a> Proof<'a, '_> {
    fn prefix(&self, syntax: &syn::PatSlice) -> Result<CheckedPrefix<'a>, String> {
        if !syntax.attrs.is_empty() || !(2..=4).contains(&syntax.elems.len()) {
            return Err(unsupported(
                "Expected one to three digits and named trailing rest",
            ));
        }

        bindings::unique_names(syntax)?;
        self.source(self.root, syntax.span())?;
        self.root_observation(syntax.elems.len() - 1)?;
        nodes::unique_ids(self.root)?;

        let count = syntax.elems.len() - 1;
        let digits = syntax
            .elems
            .iter()
            .take(count)
            .zip(&self.root.children)
            .map(|(syntax, node)| self.digit(syntax, node))
            .collect::<Result<_, _>>()?;
        let remainder = self.remainder(&syntax.elems[count], &self.root.children[count])?;

        Ok(CheckedPrefix {
            pattern: self.root,
            digits,
            remainder,
        })
    }

    fn source(&self, node: &HirNode, span: proc_macro2::Span) -> Result<(), String> {
        crate::source_evidence::verify_child_span(self.root, node, span, self.facts)
    }
}

fn unsupported(reason: &str) -> String {
    format!("Unsupported slice prefix evidence: {reason}")
}
fn unknown(reason: &str) -> String {
    format!("Unknown slice prefix evidence: {reason}")
}
