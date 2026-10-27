//! Original ordinary Copy owners retain exhaustive source/cleanup receipts.
//! A receipt supplies no callable, effect, alias or generated-body authority.
pub(super) mod source;
#[cfg(test)]
mod tests;

use super::{copy_products, product_fields::Proof, *};
use crate::nominal_boundary::resolution::{Environment, Resolution};
use copy_products::SourceReceipt;
use std::sync::Arc;

pub(in crate::nominal_boundary::resolution) struct CheckedCopyOwnerSource {
    root: TypeTree,
    declarations: Vec<copy_products::Receipt>,
    owner: copy_products::Owner,
    interface: InterfaceObservations,
    inherent: crate::inherent_calls::Definition,
    census: Arc<census::Census>,
    copy_boundary: bool,
}

/// Parent owns activation; absence/non-Copy ownership preserves existing routes.
pub(in crate::nominal_boundary::resolution) fn capture(
    facts: &HirFnFacts,
    resolution: &Resolution,
) -> Result<Option<CheckedCopyOwnerSource>, String> {
    if !source::needed(facts, resolution)? {
        return Ok(None);
    }
    let Some(inherent) = source::observation(facts)? else {
        return Ok(None);
    };
    let Some(resolved) = source::resolve(facts, resolution, &inherent)? else {
        return Ok(None);
    };
    if !source::needed_namespace(resolution, &resolved.namespace) {
        return Ok(None);
    }
    let root = resolved.root;
    if !copy_products::supported(&root) {
        return Ok(None);
    }
    let origins = facts.build.frontend_options.data::<Origins>()?;
    let proof = Proof::copy_owner(facts, resolution, &origins);
    let key = proof.declaration(&root)?;
    let attributes = super::super::declaration_attributes(proof.item(&key)?, &root)?;
    if !super::super::checked_copy(facts, &origins, &root, attributes)? {
        return Ok(None);
    }

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    if !source::interface(facts, &interface, &root)? {
        return Ok(None);
    }
    let declarations = proof.copy_receipts(&root)?;
    let owner = copy_products::Owner::capture(facts, &resolution.environment, &origins)?;
    let checked = CheckedCopyOwnerSource {
        root,
        declarations,
        owner,
        interface: interface.into_owned(),
        inherent,
        census: resolution.environment.census_owner(),
        copy_boundary: resolution.environment.copy_boundary,
    };
    checked.lexical_bindings(facts, &resolution.environment)?;
    Ok(Some(checked))
}

impl CheckedCopyOwnerSource {
    fn lexical_bindings(
        &self,
        facts: &HirFnFacts,
        environment: &Environment,
    ) -> Result<(), String> {
        super::super::imports::verify_source(
            facts,
            environment,
            self.owner.namespace(),
            self.root(),
        )
    }

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

impl SourceReceipt for CheckedCopyOwnerSource {
    fn verify_owner(&self, facts: &HirFnFacts, environment: &Environment) -> Result<(), String> {
        self.owner.verify(facts, environment)?;
        let interface = facts
            .build
            .frontend_options
            .data::<InterfaceObservations>()?;

        if interface.as_ref() != &self.interface
            || source::observation(facts)?.as_ref() != Some(&self.inherent)
            || environment.census() != self.census.as_ref()
            || environment.copy_boundary != self.copy_boundary
        {
            return Err(
                "Copy owner source receipt belongs to another interface/source boundary".into(),
            );
        }
        if !source::interface(facts, &interface, &self.root)? {
            return Err("Copy owner receipt no longer has its bounded original interface".into());
        }
        Ok(())
    }

    fn owner(&self) -> &copy_products::Owner {
        &self.owner
    }
    fn root_tree(&self) -> &TypeTree {
        &self.root
    }
    fn receipts(&self) -> &[copy_products::Receipt] {
        &self.declarations
    }
}
