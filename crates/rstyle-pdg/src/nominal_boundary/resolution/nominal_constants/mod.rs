//! Original associated-constant source proof over the already checked source owner.
mod bindings;
mod selection;
mod use_site;

use super::*;
use crate::{
    HirNode,
    data_library::Callee,
    nominal_constants::{
        Authority, BuildContext, CoreConstant, ProductConstant, SourceSelection, SourceSite,
        VerifiedValue,
    },
    unit_store::EvidenceFacts,
};

/// Compiler/target and selected-definition authority supplied by the unit coordinator.
/// These obligations cannot be reconstructed from source spelling or opaque hashes.
pub(crate) trait Context {
    fn build(&self, facts: &HirFnFacts) -> Result<(), String>;
    fn local(
        &self,
        definition: &Callee,
        source: &SourceSite,
        facts: &HirFnFacts,
    ) -> Result<(), String>;
    fn core_constant(&self, constant: &CoreConstant, facts: &HirFnFacts) -> Result<(), String>;
}

pub(crate) fn verify(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    resolution: &Resolution,
    context: &dyn Context,
) -> Result<VerifiedValue, String> {
    resolution.environment.check_context(facts.facts())?;
    crate::nominal_constants::verify(
        node,
        &SourceAuthority {
            facts,
            resolution,
            context,
        },
    )
}

struct SourceAuthority<'a> {
    facts: &'a dyn EvidenceFacts,
    resolution: &'a Resolution,
    context: &'a dyn Context,
}

impl Authority for SourceAuthority<'_> {
    fn actual(&self, node: &HirNode, record: &ProductConstant) -> Result<(), String> {
        use_site::verify(node, record, self)
    }

    fn build(&self) -> Result<BuildContext<'_>, String> {
        let facts = self.facts.facts();
        self.context.build(facts)?;
        if facts.build.pointer_width.status != crate::FactStatus::Known {
            return Err("Product constant source requires known target pointer width".into());
        }
        Ok(BuildContext {
            pointer_width: facts
                .build
                .pointer_width
                .value
                .ok_or("Missing product constant pointer width")?,
            cfgs: &facts.build.cfgs,
        })
    }

    fn sources(&self, record: &ProductConstant) -> Result<SourceSelection<'_>, String> {
        selection::sources(self.resolution, record)
    }

    fn local(&self, definition: &Callee, source: &SourceSite) -> Result<(), String> {
        self.context.local(definition, source, self.facts.facts())
    }

    fn bindings(
        &self,
        record: &ProductConstant,
        source: &SourceSelection<'_>,
    ) -> Result<(), String> {
        bindings::verify(self, record, source)
    }

    fn owned_copy(&self, record: &ProductConstant) -> Result<(), String> {
        if !self.resolution.checked_owned_copy(&record.ty) {
            return Err(
                "Product constant lacks complete original nominal Copy source authority".into(),
            );
        }
        let source = selection::owner_declaration(self.resolution, record)?;

        self.context.local(
            &record.constructor.tuple_constructor,
            &source,
            self.facts.facts(),
        )
    }

    fn core_constant(&self, constant: &CoreConstant) -> Result<(), String> {
        self.context.core_constant(constant, self.facts.facts())
    }
}
