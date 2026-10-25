//! Source-only expression inventory; never callee, purity or cleanup authority.
mod arguments;
mod source;
#[cfg(test)]
mod tests;

use crate::{
    dependency_namespaces::capture::{Export, UnitCapture},
    generated_sources::capture::Definition,
    nominal_boundary::census::{Census, expansions::FileSpan},
};
use std::sync::Arc;
pub(crate) const CAPABILITY: &str = "rust.checked-external-expression-macro-ownership";

pub(crate) const CORRECTED_CAPABILITY: &str = "rust.checked-corrected-expression-macro-ownership";

#[derive(Clone, Copy)]
pub(crate) struct Review<'a> {
    pub cfgs: &'a [String],
    pub corrected: bool,
}

/// All observations must belong to the caller's independently associated
/// producer. This component cannot establish compiled artifact ownership.
pub(crate) struct Input<'a> {
    pub census: Arc<Census>,
    pub namespaces: &'a UnitCapture,
    pub export: &'a Export,
    /// Exact original complete macro item, including its cfg/export attributes.
    pub definition: &'a FileSpan,
    pub review: Review<'a>,
}

/// Private construction requires checked original bytes and namespace joins.
/// Callers separately prove the consumer path resolves to `definition()`.
pub(crate) struct Receipt {
    census: Arc<Census>,
    definition: Definition,
}

impl Receipt {
    pub(crate) fn definition(&self) -> &Definition {
        &self.definition
    }

    pub(crate) fn census(&self) -> &Arc<Census> {
        &self.census
    }

    /// Visit every returned expression with the exhaustive ownership visitor.
    /// The invocation's resolved identity must first match this receipt.
    pub(crate) fn expressions(
        &self,
        invocation: &syn::Macro,
        resolved: &Definition,
    ) -> Result<Vec<syn::Expr>, String> {
        if &self.definition != resolved {
            return Err("Invocation does not resolve to the source-checked external macro".into());
        }
        bounded_tokens(invocation.tokens.clone(), 0, &mut 131_072)?;
        syn::parse2::<arguments::Arguments>(invocation.tokens.clone())
            .map(|args| args.0)
            .map_err(|error| error.to_string())
    }
}

fn bounded_tokens(
    tokens: proc_macro2::TokenStream,
    depth: usize,
    remaining: &mut usize,
) -> Result<(), String> {
    if depth > 48 {
        return Err("External expression macro nesting budget exceeded".into());
    }
    for token in tokens {
        *remaining = remaining
            .checked_sub(1)
            .ok_or("External expression macro work budget exceeded")?;
        if let proc_macro2::TokenTree::Group(group) = token {
            bounded_tokens(group.stream(), depth + 1, remaining)?;
        }
    }
    Ok(())
}

pub(crate) fn verify(input: Input<'_>) -> Result<Receipt, String> {
    source::verify(&input)?;
    Ok(Receipt {
        census: input.census,
        definition: input.export.target.clone(),
    })
}

/// Derive the complete original item span from the reviewed producer module.
/// The caller must already hold this producer through a checked dependency edge.
pub(crate) fn from_owner(
    census: Arc<Census>,
    namespaces: &UnitCapture,
    export: &Export,
    review: Review<'_>,
) -> Result<Receipt, String> {
    let definition = source::definition(&census, review)?;
    verify(Input {
        census,
        namespaces,
        export,
        definition: &definition,
        review,
    })
}
