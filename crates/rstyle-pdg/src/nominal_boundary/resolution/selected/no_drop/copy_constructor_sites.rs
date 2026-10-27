//! Exact original Copy constructors; no whole-unit source-closure authority.
mod owner;
mod shape;
#[cfg(test)]
mod tests;

use super::*;
use crate::nominal_boundary::resolution::{Namespace, Resolution};

/// This stack-local receipt belongs to one immutable input, declaration and
/// lexical site. It must never stand in for an exhaustive namespace certificate.
pub(in crate::nominal_boundary::resolution) struct CheckedSite<'a> {
    facts: &'a HirFnFacts,
    tree: TypeTree,
    namespace: Namespace,
}

impl CheckedSite<'_> {
    pub(in crate::nominal_boundary::resolution) fn permits(
        &self,
        facts: &HirFnFacts,
        tree: &TypeTree,
        namespace: &[String],
    ) -> bool {
        std::ptr::eq(self.facts, facts) && self.tree == *tree && self.namespace == namespace
    }

    /// Positive Copy proves target cleanup only. This does not prove imports,
    /// unrelated items, arbitrary callable bodies or global expansion absence.
    pub(in crate::nominal_boundary::resolution) fn cleanup(&self) -> CheckedNoDrop {
        CheckedNoDrop {
            nominal: self
                .tree
                .nominal_parts()
                .expect("Checked constructor nominal")
                .0
                .clone(),
        }
    }
}

pub(in crate::nominal_boundary::resolution) fn capture<'a>(
    facts: &'a HirFnFacts,
    resolution: &Resolution,
    origins: &Origins,
    tree: &TypeTree,
) -> Result<CheckedSite<'a>, String> {
    let proof = product_fields::Proof::copy_product(facts, resolution, origins);
    let key = proof.declaration(tree)?;
    let item = proof.item(&key)?;
    let syn::Item::Struct(item) = item else {
        return Err("Copy constructor requires an original concrete struct".into());
    };

    shape::declaration(item, tree)?;
    super::super::structure(facts, item, true)?;

    if !super::super::checked_copy(facts, origins, tree, &item.attrs)? {
        return Err("Copy constructor lacks exact positive builtin Copy evidence".into());
    }

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;

    if !matches!(&interface.result, Observation::Captured { ty } if ty == tree) {
        return Err("Copy constructor result differs from its complete captured interface".into());
    }

    if !matches!(interface.parameters.as_slice(), [Observation::Captured { ty }]
        if ty == &TypeTree::Shared(Box::new(TypeTree::Str)))
    {
        return Err(
            "Copy constructor requires its complete static string input observation".into(),
        );
    }

    resolution.verify_transport_origins(&interface, origins, facts)?;
    let function = crate::data_frontend::source_function(facts, Some(resolution))?;
    shape::function(&function, facts, tree)?;
    let namespace = owner::verify(facts, resolution, tree)?;
    crate::source_evidence::verify(facts)?;

    Ok(CheckedSite {
        facts,
        tree: tree.clone(),
        namespace,
    })
}
