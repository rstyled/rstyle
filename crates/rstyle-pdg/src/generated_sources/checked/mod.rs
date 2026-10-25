//! Independent source ownership only; body effects and callable permission stay separate.
mod authority;
mod roles;
mod roster;
#[cfg(test)]
pub(crate) mod tests;

use super::capture::*;
use crate::{
    HirFnFacts, HirRef,
    nominal_boundary::census::{Census, expansions::FileSpan, generated_sources as source},
};
use syn::spanned::Spanned;

pub(crate) struct Input<'a> {
    pub facts: &'a HirFnFacts,
    pub census: &'a Census,
    pub capture: &'a UnitCapture,
}

pub(crate) struct CheckedMember {
    owner: Definition,
    defining_expansion: String,
    expansion: source::CheckedExpansion,
    implementation: usize,
    member: usize,
    roles: Vec<CheckedRole>,
}

pub(crate) struct CheckedRole {
    path: Vec<u32>,
    hir: HirRef,
    kind: String,
    source: FileSpan,
}

impl CheckedRole {
    pub(crate) fn path(&self) -> &[u32] {
        &self.path
    }
    pub(crate) fn hir(&self) -> &HirRef {
        &self.hir
    }
    pub(crate) fn kind(&self) -> &str {
        &self.kind
    }
    pub(crate) fn source(&self) -> &FileSpan {
        &self.source
    }
}

impl CheckedMember {
    pub(crate) fn owner(&self) -> &Definition {
        &self.owner
    }
    pub(crate) fn defining_expansion(&self) -> &str {
        &self.defining_expansion
    }

    pub(crate) fn function(&self) -> &syn::ImplItemFn {
        let syn::ImplItem::Fn(function) = &self.implementation().items[self.member] else {
            unreachable!("Checked function ordinal")
        };
        function
    }
    pub(crate) fn implementation(&self) -> &syn::ItemImpl {
        let syn::Item::Impl(item) = &self.expansion.items()[self.implementation] else {
            unreachable!("Checked implementation ordinal")
        };
        item
    }
    pub(crate) fn namespace(&self) -> &[String] {
        self.expansion.namespace()
    }
    pub(crate) fn expansion(&self) -> &source::CheckedExpansion {
        &self.expansion
    }
    pub(crate) fn roles(&self) -> &[CheckedRole] {
        &self.roles
    }
}

#[cfg(test)]
pub(crate) fn verify(input: Input<'_>) -> Result<CheckedMember, String> {
    verify_using(input, source::verify_compiler)
}

pub(crate) fn verify_in(
    input: Input<'_>,
    environment: &crate::nominal_boundary::resolution::Environment,
) -> Result<CheckedMember, String> {
    verify_using(input, |source| {
        source::verify_compiler_in(source, environment)
    })
}

fn verify_using(
    input: Input<'_>,
    expansion: impl FnOnce(source::CompilerInput<'_>) -> Result<source::CheckedExpansion, String>,
) -> Result<CheckedMember, String> {
    let (observation, root) = authority::selected(&input)?;
    let expansion = expansion(source::CompilerInput {
        census: input.census,
        cfgs: &input.facts.build.cfgs,
        definition_header: root
            .definition
            .as_ref()
            .ok_or("Missing local macro definition header")?,
        invocation_macro: &root.invocation.raw,
        preceding_expansions: crate::extraction_profile::facts_support(
            input.facts,
            "rust.ordered-local-macro-source",
        ),
    })?;
    authority::source(&input, observation, root, &expansion)?;

    let selection = roster::verify(&input, observation, &expansion)?;
    let function = selection.function(&expansion);
    let roles = roles::verify(&input, observation, function, &expansion)?;
    Ok(CheckedMember {
        owner: observation.owner.clone(),
        defining_expansion: observation.defining_expansion.clone(),
        expansion,
        implementation: selection.implementation,
        member: selection.member,
        roles,
    })
}

fn origin(origin: &Origin, expected: &FileSpan, root: &str) -> Result<(), String> {
    if origin.raw != *expected
        || origin.marks
            != [Mark {
                expansion: root.into(),
                transparency: Transparency::SemiOpaque,
            }]
    {
        return Err(
            "Generated source anchor or exact hygiene differs from original template".into(),
        );
    }
    Ok(())
}

fn visible_span(
    vis: &syn::Visibility,
    span: proc_macro2::Span,
) -> Result<proc_macro2::Span, String> {
    if matches!(vis, syn::Visibility::Inherited) {
        return Ok(span);
    }
    vis.span()
        .join(span)
        .ok_or_else(|| "Generated source header has noncontiguous coordinates".into())
}
