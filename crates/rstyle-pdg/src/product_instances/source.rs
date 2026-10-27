//! Complete original Match source roles; this token grants no instance authority.
mod body;
mod owners;
mod roster;
mod signature;
#[cfg(test)]
mod tests;

use crate::{HirFnFacts, HirNode, concrete_types::TypeTree};
use serde_json::Value;
use syn::spanned::Spanned;

type Result<T> = std::result::Result<T, String>;

pub(super) struct CheckedSource<'a> {
    facts: &'a HirFnFacts,
    pattern: &'a HirNode,
    input: &'a HirNode,
    local: &'a HirNode,
    conversion: &'a HirNode,
    assertion: &'a HirNode,
    result: &'a HirNode,
}

impl<'a> CheckedSource<'a> {
    pub(super) fn facts(&self) -> &'a HirFnFacts {
        self.facts
    }
    pub(super) fn pattern(&self) -> &'a HirNode {
        self.pattern
    }
    pub(super) fn input(&self) -> &'a HirNode {
        self.input
    }
    pub(super) fn local(&self) -> &'a HirNode {
        self.local
    }
    pub(super) fn conversion(&self) -> &'a HirNode {
        self.conversion
    }
    pub(super) fn receiver(&self) -> &'a HirNode {
        &self.conversion.children[0]
    }
    pub(super) fn assertion(&self) -> &'a HirNode {
        self.assertion
    }
    pub(super) fn condition(&self) -> &'a HirNode {
        &self.assertion.children[0]
    }
    pub(super) fn endpoints(&self) -> [&'a HirNode; 2] {
        [&self.condition().children[0], &self.condition().children[1]]
    }
    pub(super) fn result(&self) -> &'a HirNode {
        self.result
    }
    pub(super) fn result_operands(&self) -> [&'a HirNode; 2] {
        [&self.result.children[0], &self.result.children[1]]
    }
}

pub(super) fn verify<'a>(
    facts: &'a HirFnFacts,
    observation: &Value,
    result: &TypeTree,
) -> Result<CheckedSource<'a>> {
    use crate::extraction_profile::ExtractionProfile;

    if ExtractionProfile::from_facts(facts)? != ExtractionProfile::DataV3
        || facts.source.bytes.len() > crate::concrete_types::MAX_SOURCE_BYTES
    {
        return Err("Product source proof requires bounded original DataV3 facts".into());
    }
    crate::nominal_boundary::resolution::verify_source_inputs(facts)?;
    crate::source_evidence::verify(facts)?;
    let syntax: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;
    signature::verify(facts, &syntax, result)?;
    let checked = body::verify(facts, &syntax)?;
    owners::verify(facts, observation)?;
    roster::verify(&checked, observation)?;
    Ok(checked)
}

fn span(facts: &HirFnFacts, syntax: &impl Spanned) -> crate::ByteSpan {
    crate::source_evidence::original_span(&facts.source.bytes, syntax.span())
}

struct Shape {
    kind: &'static str,
    children: usize,
    typed: bool,
}

fn node(facts: &HirFnFacts, node: &HirNode, syntax: &impl Spanned, shape: Shape) -> Result<()> {
    let expected = span(facts, syntax);

    if node.kind != shape.kind
        || node.children.len() != shape.children
        || node.span != expected
        || node.ty.is_some() != shape.typed
        || node.is_unsafe_block
        || node.hir.krate != 0
        || node.hir.owner != facts.body.hir.owner
        || crate::source_evidence::byte_range(&facts.source.bytes, expected)? != node.snippet
    {
        return Err("Product source/HIR node roster or exact original bytes disagree".into());
    }
    Ok(())
}

fn plain(node: &HirNode) -> Result<()> {
    if node.binding.is_some()
        || node.binding_mut.is_some()
        || node.param_index.is_some()
        || node.lit.is_some()
        || node.expansion.is_some()
        || node.dispatch.is_some()
        || node.res.is_some()
    {
        return Err("Product node adds unproved optional evidence".into());
    }
    Ok(())
}

fn path(facts: &HirFnFacts, syntax: &syn::Expr, node: &HirNode, binding: &HirNode) -> Result<()> {
    let syn::Expr::Path(path) = syntax else {
        return Err("Product read is not a local path".into());
    };

    node_read(facts, node, path, binding)?;
    if path.qself.is_some() || !path.attrs.is_empty() || !path.path.is_ident(&binding.snippet) {
        return Err("Product read source changes its saved binding".into());
    }
    Ok(())
}

fn node_read(
    facts: &HirFnFacts,
    read: &HirNode,
    syntax: &impl Spanned,
    binding: &HirNode,
) -> Result<()> {
    node(
        facts,
        read,
        syntax,
        Shape {
            kind: "expr_path",
            children: 0,
            typed: true,
        },
    )?;

    let mut without = read.clone();
    without.res = None;
    plain(&without)?;
    if read
        .res
        .as_ref()
        .is_none_or(|res| res.kind != "local" || res.target.as_ref() != Some(&binding.hir))
        || !same_value_type(read, binding)
    {
        return Err("Product read loses its original saved-value target/type".into());
    }
    Ok(())
}

fn same_value_type(read: &HirNode, binding: &HirNode) -> bool {
    match (&read.ty, &binding.ty) {
        (Some(read), Some(binding)) => {
            read.normalized == binding.normalized
                && read.observed == binding.observed
                && read.spelling.is_empty()
        }
        _ => false,
    }
}
