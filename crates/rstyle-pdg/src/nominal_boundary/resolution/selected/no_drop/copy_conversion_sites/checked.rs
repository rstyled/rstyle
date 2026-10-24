//! Target-specific source and core membership retained before selection.
use super::*;
use std::sync::Arc;

mod binding;
#[cfg(test)]
mod tests;

/// No global namespace, arbitrary method, value or caller-summary authority.
pub(crate) struct CheckedConversion {
    source: CheckedSource,
    binding: binding::Binding,
    owner: copy_products::Owner,
    census: Arc<census::Census>,
    copy_boundary: bool,
}

pub(in crate::nominal_boundary::resolution) fn capture_checked(
    facts: &HirFnFacts,
    resolution: &Resolution,
    origins: &Origins,
) -> Result<CheckedConversion, String> {
    let source = capture(facts, resolution, origins)?;
    verify_member(facts, &source)?;
    let owner = copy_products::Owner::capture_at(
        facts,
        &resolution.environment,
        origins,
        source.namespace.clone(),
    )?;

    Ok(CheckedConversion {
        source,
        binding: binding::Binding::capture(facts)?,
        owner,
        census: resolution.environment.census_owner(),
        copy_boundary: resolution.environment.copy_boundary,
    })
}

impl CheckedConversion {
    pub(crate) fn input(&self) -> &TypeTree {
        self.source.endpoints.input()
    }

    pub(crate) fn output(&self) -> &TypeTree {
        self.source.endpoints.output()
    }

    pub(crate) fn product(&self) -> &TypeTree {
        self.source.endpoints.product()
    }

    pub(crate) fn tagged(&self) -> &TypeTree {
        self.source.endpoints.tagged()
    }

    pub(crate) fn error(&self) -> Option<&TypeTree> {
        self.source.endpoints.error()
    }

    pub(in crate::nominal_boundary::resolution) fn checked_namespace<'a>(
        &'a self,
        facts: &HirFnFacts,
        environment: &Environment,
    ) -> Result<&'a [String], String> {
        self.owner.verify(facts, environment)?;
        if !Arc::ptr_eq(&self.census, &environment.census_owner())
            || self.copy_boundary != environment.copy_boundary
        {
            return Err(
                "Copy conversion belongs to another exact checked source environment".into(),
            );
        }
        self.binding.verify(facts)?;
        Ok(self.owner.namespace())
    }

    pub(in crate::nominal_boundary::resolution) fn self_type(&self) -> &TypeTree {
        match self.source.endpoints.direction {
            Direction::Wrap => self.source.endpoints.tagged(),
            Direction::Unwrap => self.source.endpoints.product(),
        }
    }

    pub(in crate::nominal_boundary::resolution) fn declaration(
        &self,
        tree: &TypeTree,
    ) -> Option<&CheckedNoDrop> {
        self.target(tree).map(|target| &target.cleanup)
    }

    pub(in crate::nominal_boundary::resolution) fn permits_type(&self, tree: &TypeTree) -> bool {
        self.target(tree).is_some()
    }

    pub(in crate::nominal_boundary::resolution) fn declaration_namespace(
        &self,
        tree: &TypeTree,
        namespace: &[String],
    ) -> Result<bool, String> {
        targets::namespace(&self.source.targets, tree, namespace)
    }

    pub(in crate::nominal_boundary::resolution) fn function_namespace(
        &self,
        facts: &HirFnFacts,
        environment: &Environment,
        namespace: &[String],
    ) -> Result<(), String> {
        if self.checked_namespace(facts, environment)? != namespace {
            return Err(
                "Copy conversion function differs from its exact original method namespace".into(),
            );
        }
        Ok(())
    }

    fn target(&self, tree: &TypeTree) -> Option<&targets::Target> {
        targets::find(&self.source.targets, tree)
    }
}

fn verify_member(facts: &HirFnFacts, source: &CheckedSource) -> Result<(), String> {
    use crate::copy_conversion_members::{Contract, Direction as MemberDirection};
    let endpoints = source.endpoints();
    let direction = match endpoints.direction() {
        Direction::Wrap => MemberDirection::Wrap,
        Direction::Unwrap => MemberDirection::Unwrap,
    };
    crate::copy_conversion_members::verify(
        facts,
        Contract {
            direction,
            input: endpoints.input(),
            output: endpoints.output(),
            implementation_source: source.implementation(),
            method_source: source.method(),
            error_source: source.associated_error(),
        },
    )
}
