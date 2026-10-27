//! Independent success Copy and error no-drop receipts before permission selection.
//! Source closure proves neither a callable body nor its effects.
mod source;
#[cfg(test)]
mod tests;

use super::{copy_products, product_fields::Proof, *};
use crate::nominal_boundary::resolution::Resolution;
use std::sync::Arc;

pub(in crate::nominal_boundary::resolution) struct CheckedScalarResultSource {
    success: TypeTree,
    error: TypeTree,
    copies: Vec<copy_products::Receipt>,
    cleanup: CheckedNoDrop,
    owner: copy_products::Owner,
    binding: source::Binding,
    interface: InterfaceObservations,
    census: Arc<census::Census>,
    copy_boundary: bool,
}

pub(in crate::nominal_boundary::resolution) fn capture(
    facts: &HirFnFacts,
    resolution: &Resolution,
) -> Result<Option<CheckedScalarResultSource>, String> {
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Some((success, error)) = source::interface(facts, &interface)? else {
        return Ok(None);
    };
    let Some(binding) = source::bind(facts, resolution, success, error)? else {
        return Ok(None);
    };
    let origins = facts.build.frontend_options.data::<Origins>()?;
    resolution.verify_transport_origins(&interface, &origins, facts)?;
    super::super::super::verify_declarations(resolution, &origins, facts)?;
    let copies = Proof::copy_owner(facts, resolution, &origins).copy_receipts(success)?;
    let cleanup = super::verify(facts, error, &resolution.environment)?;
    let owner = copy_products::Owner::capture_at(
        facts,
        &resolution.environment,
        &origins,
        binding.namespace().to_vec(),
    )?;
    let root = copies
        .iter()
        .find(|receipt| receipt.tree() == success)
        .ok_or("Scalar Result success lacks its exact Copy source receipt")?;
    super::super::imports::verify_source(
        facts,
        &resolution.environment,
        binding.namespace(),
        root.certificate(),
    )?;
    binding.lexical(facts, resolution, root.certificate())?;

    Ok(Some(CheckedScalarResultSource {
        success: success.clone(),
        error: error.clone(),
        copies,
        cleanup,
        owner,
        binding,
        interface: interface.into_owned(),
        census: resolution.environment.census_owner(),
        copy_boundary: resolution.environment.copy_boundary,
    }))
}

impl CheckedScalarResultSource {
    pub(in crate::nominal_boundary::resolution) fn checked_namespace<'a>(
        &'a self,
        facts: &HirFnFacts,
        environment: &Environment,
    ) -> Result<&'a [String], String> {
        let namespace = self.owner.namespace();
        self.function_namespace(facts, environment, namespace)?
            .ok_or("Scalar Result receipt lacks its exact function namespace")?;
        Ok(namespace)
    }

    pub(in crate::nominal_boundary::resolution) fn trait_member(
        &self,
    ) -> Option<&crate::tryfrom_calls::SourceMember> {
        match &self.binding {
            source::Binding::TryFrom { member, .. } => Some(member),
            source::Binding::Inherent { .. } => None,
        }
    }

    pub(in crate::nominal_boundary::resolution) fn trait_owner(&self) -> bool {
        matches!(&self.binding, source::Binding::TryFrom { .. })
    }

    pub(in crate::nominal_boundary::resolution) fn verify_builtin_imports(
        &self,
        facts: &HirFnFacts,
        environment: &Environment,
        namespace: &[String],
    ) -> Result<(), String> {
        let certificate = self
            .function_namespace(facts, environment, namespace)?
            .ok_or("Builtin Result scope lacks its exact scalar Result receipt")?;
        super::super::imports::verify_source(facts, environment, namespace, certificate)
    }

    pub(in crate::nominal_boundary::resolution) fn declaration(
        &self,
        tree: &TypeTree,
    ) -> Option<&CheckedNoDrop> {
        if tree == &self.error {
            return Some(&self.cleanup);
        }
        self.copies
            .iter()
            .find(|receipt| receipt.tree() == tree)
            .map(copy_products::Receipt::certificate)
    }

    pub(in crate::nominal_boundary::resolution) fn function_namespace<'a>(
        &'a self,
        facts: &HirFnFacts,
        environment: &Environment,
        namespace: &[String],
    ) -> Result<Option<&'a CheckedNoDrop>, String> {
        self.owner.verify(facts, environment)?;
        self.binding.verify(facts)?;
        let interface = facts
            .build
            .frontend_options
            .data::<InterfaceObservations>()?;
        if interface.as_ref() != &self.interface
            || environment.census() != self.census.as_ref()
            || environment.copy_boundary != self.copy_boundary
        {
            return Err(
                "Scalar Result receipt belongs to another interface/source boundary".into(),
            );
        }
        if self.owner.namespace() != namespace {
            return Ok(None);
        }

        Ok(self.declaration(&self.success))
    }
}
