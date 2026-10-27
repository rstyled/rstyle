//! Complete source/cleanup proof for bounded active scalar Copy products.
//! No instance, function-effect or value permission follows from a probe.
mod owner;
mod receipts;

use super::product_fields::Proof;
use super::*;
use crate::nominal_boundary::resolution::{Resolution, verify_declarations};
pub(super) use owner::{Owner, SourceContext};
pub(super) use receipts::{Receipt, SourceReceipt};
use std::sync::Arc;

pub(in crate::nominal_boundary::resolution::selected) fn supported(tree: &TypeTree) -> bool {
    matches!(tree, TypeTree::Struct { .. }) && shape(tree, 0)
}

fn shape(tree: &TypeTree, depth: usize) -> bool {
    if depth > concrete_types::MAX_DEPTH {
        return false;
    }
    match tree {
        TypeTree::Scalar(token) => concrete_types::scalar(token) && token != "unit",
        TypeTree::Struct { source, fields, .. } => {
            !fields.is_empty()
                && fields.len() <= concrete_types::MAX_FIELDS
                && syn::parse_str::<syn::ItemStruct>(source).is_ok_and(|item| {
                    item.generics.params.is_empty() && item.generics.where_clause.is_none()
                })
                && fields.iter().all(|field| shape(&field.ty, depth + 1))
        }
        _ => false,
    }
}

/// The caller owns capability activation; all nominal fields need actual proof.
pub(in crate::nominal_boundary::resolution) fn verify(
    facts: &HirFnFacts,
    tree: &TypeTree,
    resolution: &Resolution,
    origins: &Origins,
) -> Result<CheckedNoDrop, String> {
    inputs(facts, tree, resolution, origins)?;
    let checker = Proof::copy_product(facts, resolution, origins);
    checker.copy(tree)?;

    let (nominal, _) = tree
        .nominal_parts()
        .ok_or("Missing Copy cleanup product identity")?;
    Ok(CheckedNoDrop {
        nominal: nominal.clone(),
    })
}

#[cfg(test)]
mod tests;

/// Immutable receipts from complete recursive Copy/source checks, never flags.
pub(crate) struct CheckedCopyProductSource {
    root: TypeTree,
    declarations: Vec<Receipt>,
    owner: Owner,
    census: Arc<crate::nominal_boundary::census::Census>,
    copy_boundary: bool,
}

pub(crate) fn capture(
    facts: &HirFnFacts,
    resolution: &Resolution,
) -> Result<CheckedCopyProductSource, String> {
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured { ty: root } = &interface.result else {
        return Err("Copy product source requires the complete captured result".into());
    };
    let origins = facts.build.frontend_options.data::<Origins>()?;

    inputs(facts, root, resolution, &origins)?;
    let declarations = Proof::copy_product(facts, resolution, &origins).copy_receipts(root)?;
    let owner = Owner::capture(facts, &resolution.environment, &origins)?;
    Ok(CheckedCopyProductSource {
        root: root.clone(),
        declarations,
        owner,
        census: resolution.environment.census_owner(),
        copy_boundary: resolution.environment.copy_boundary,
    })
}

impl CheckedCopyProductSource {
    pub(in crate::nominal_boundary::resolution) fn declaration(
        &self,
        tree: &TypeTree,
    ) -> Option<&CheckedNoDrop> {
        SourceReceipt::declaration(self, tree)
    }

    pub(in crate::nominal_boundary::resolution) fn function_namespace<'a>(
        &'a self,
        facts: &HirFnFacts,
        environment: &Environment,
        namespace: &[String],
    ) -> Result<Option<&'a CheckedNoDrop>, String> {
        SourceReceipt::function_namespace(self, facts, environment, namespace)
    }
}

fn inputs(
    facts: &HirFnFacts,
    tree: &TypeTree,
    resolution: &Resolution,
    origins: &Origins,
) -> Result<(), String> {
    concrete_types::validate(tree)?;
    if !supported(tree) {
        return Err(
            "Copy cleanup product requires complete lifetime-free scalar/struct fields".into(),
        );
    }

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    crate::declaration_origins::verify_in(
        &interface,
        origins,
        &resolution.environment,
        &facts.build.frontend_options,
    )?;
    verify_declarations(resolution, origins, facts)?;
    Ok(())
}

impl SourceReceipt for CheckedCopyProductSource {
    fn verify_owner(&self, facts: &HirFnFacts, environment: &Environment) -> Result<(), String> {
        self.owner.verify(facts, environment)?;

        if environment.census() != self.census.as_ref()
            || environment.copy_boundary != self.copy_boundary
        {
            return Err(
                "Copy product source belongs to another immutable source environment".into(),
            );
        }

        let interface = facts
            .build
            .frontend_options
            .data::<InterfaceObservations>()?;
        if !matches!(&interface.result, Observation::Captured { ty } if ty == &self.root) {
            return Err("Copy product source result differs from its complete checked root".into());
        }
        Ok(())
    }

    fn owner(&self) -> &Owner {
        &self.owner
    }
    fn root_tree(&self) -> &TypeTree {
        &self.root
    }
    fn receipts(&self) -> &[Receipt] {
        &self.declarations
    }
}

pub(in crate::nominal_boundary::resolution) fn verify_owner(
    source: &CheckedCopyProductSource,
    facts: &HirFnFacts,
    environment: &Environment,
) -> Result<(), String> {
    SourceReceipt::verify_owner(source, facts, environment)
}
