//! Positive Copy/source proof with an exact, private static-string product receipt.
mod shape;
mod source;
#[cfg(test)]
mod tests;

use super::product_fields::Proof;
use super::*;
use crate::nominal_boundary::resolution::Resolution;

pub(in crate::nominal_boundary::resolution) const CAPABILITY: &str =
    "rust.static-string-copy-products";

/// Regions are erased in TypeTree. Only the original declaration, complete
/// recursive Copy proof and source closure may mint this local permission.
pub(in crate::nominal_boundary::resolution) struct CheckedProduct(TypeTree);

impl CheckedProduct {
    pub(in crate::nominal_boundary::resolution) fn permits(&self, tree: &TypeTree) -> bool {
        &self.0 == tree
    }

    pub(in crate::nominal_boundary::resolution) fn cleanup(&self) -> CheckedNoDrop {
        CheckedNoDrop {
            nominal: self
                .0
                .nominal_parts()
                .expect("Checked product nominal")
                .0
                .clone(),
        }
    }
}

pub(in crate::nominal_boundary::resolution) fn required(tree: &TypeTree) -> bool {
    let TypeTree::Struct { source, fields, .. } = tree else {
        return false;
    };

    // Ordinary scalar products take the typed fast path without parsing source.
    fields.iter().any(|field| {
        matches!(&field.ty, TypeTree::Shared(inner) if inner.as_ref() == &TypeTree::Str)
            || required(&field.ty)
    }) && concrete_types::lifetimes::declaration_count(source) == Ok(0)
}

pub(in crate::nominal_boundary::resolution) fn verify_product(
    facts: &HirFnFacts,
    resolution: &Resolution,
    origins: &Origins,
    tree: &TypeTree,
) -> Result<Option<CheckedProduct>, String> {
    if !required(tree) {
        return Ok(None);
    }

    if !supports(facts, CAPABILITY)? || !matches!(tree, TypeTree::Struct { .. }) {
        return Err(
            "Static string products require their registered concrete struct contract".into(),
        );
    }
    if !shape::supported(tree) {
        return Err("Static string product has unsupported complete components".into());
    }

    let mut checker = Checker {
        facts,
        origins,
        proof: Proof::copy_product(facts, resolution, origins),
        checked: Vec::new(),
    };
    checker.nominal(tree)?;

    let function = crate::data_frontend::source_function(facts, Some(resolution))?;
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    source::parameters(&function, &interface)?;
    Ok(Some(CheckedProduct(tree.clone())))
}

/// Every interface observation must be complete. Result carriers are inspected
/// recursively; their concrete nominal children need their own positive proof.
pub(in crate::nominal_boundary::resolution) fn verify_interface(
    facts: &HirFnFacts,
    resolution: &Resolution,
    origins: &Origins,
) -> Result<(), String> {
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let mut checker = Checker {
        facts,
        origins,
        proof: Proof::copy_product(facts, resolution, origins),
        checked: Vec::new(),
    };
    if interface.parameters.len() != facts.signature.params.len() {
        return Err("Copy component interface parameter roster disagrees".into());
    }

    let spellings = facts
        .signature
        .params
        .iter()
        .map(|param| param.ty.spelling.as_str())
        .chain([facts.signature.return_ty.spelling.as_str()]);
    for (observation, spelling) in interface
        .parameters
        .iter()
        .chain([&interface.result])
        .zip(spellings)
    {
        let Observation::Captured { ty } = observation else {
            return Err("Copy component probe requires a complete captured interface".into());
        };
        source::interface(ty, spelling)?;
        checker.interface_tree(ty)?;
    }
    Ok(())
}

struct Checker<'a> {
    facts: &'a HirFnFacts,
    origins: &'a Origins,
    proof: Proof<'a>,
    checked: Vec<TypeTree>,
}

impl Checker<'_> {
    fn interface_tree(&mut self, tree: &TypeTree) -> Result<(), String> {
        if let TypeTree::Result { ok, err } = tree {
            self.interface_tree(ok)?;
            return self.interface_tree(err);
        }
        if !shape::supported(tree) {
            return Err("Copy component requires bounded concrete nominal fields".into());
        }
        if tree.nominal_parts().is_some() {
            self.nominal(tree)?;
        }
        Ok(())
    }

    fn nominal(&mut self, tree: &TypeTree) -> Result<(), String> {
        if self.checked.contains(tree) {
            return Ok(());
        }
        let key = self.proof.declaration(tree)?;
        let item = self.proof.item(&key)?;
        source::verify(item, tree)?;

        let attributes = super::super::declaration_attributes(item, tree)?;
        if !super::super::checked_copy(self.facts, self.origins, tree, attributes)? {
            return Err("Copy component lacks its exact positive builtin Copy proof".into());
        }
        match item {
            syn::Item::Struct(item) => super::super::structure(self.facts, item, true)?,
            syn::Item::Enum(item) => super::super::scalar_enum(self.facts, item)?,
            _ => return Err("Copy component has an unsupported declaration kind".into()),
        }
        for field in tree.nominal_fields() {
            if field.ty.nominal_parts().is_some() {
                self.nominal(&field.ty)?;
            }
        }

        self.proof.closure(&key)?;
        self.checked.push(tree.clone());
        Ok(())
    }
}
