//! Concrete source/build authority for ID widths in ordinary original readers.
mod generated;
pub(super) mod memo;
mod original;
mod use_site;

use super::*;
use crate::{
    HirNode,
    generated_sources::{capture::*, checked::authority},
    id_reader_constants::{self as contract, Annotation, Observation, Site},
    unit_store::EvidenceFacts,
};

pub(crate) fn verify(
    node: &HirNode,
    input: &dyn EvidenceFacts,
    resolution: &Resolution,
) -> Result<u64, String> {
    contract::verify(node, &SourceAuthority { input, resolution })
}

pub(crate) fn verify_annotation(
    node: &HirNode,
    input: &dyn EvidenceFacts,
    resolution: &Resolution,
) -> Result<u64, String> {
    contract::verify_annotation(node, &SourceAuthority { input, resolution })
}

struct SourceAuthority<'a> {
    input: &'a dyn EvidenceFacts,
    resolution: &'a Resolution,
}

impl contract::Authority for SourceAuthority<'_> {
    fn actual(&self, node: &HirNode, record: &Observation) -> Result<(), String> {
        use_site::expression(self, node, record)
    }

    fn annotation(&self, node: &HirNode, record: &Annotation) -> Result<(), String> {
        use_site::annotation(self, node, record)
    }

    fn chain(&self, record: &Observation) -> Result<(), String> {
        self.context(record)?;
        let owner = memo::Owner::capture(self.input)?;
        if self
            .resolution
            .id_reader_constant_chains
            .borrow()
            .contains(&owner, record)?
        {
            return Ok(());
        }

        original::verify(self, record)?;
        generated::verify(self, record)?;
        self.resolution
            .id_reader_constant_chains
            .borrow_mut()
            .retain(owner, record.clone());
        Ok(())
    }
}

impl SourceAuthority<'_> {
    fn context(&self, record: &Observation) -> Result<(), String> {
        let facts = self.input.facts();
        if !crate::extraction_profile::facts_support(facts, contract::CAPABILITY) {
            return Err("ID reader byte widths require their registered capability".into());
        }
        self.resolution.environment.check_context(facts)?;
        self.resolution.environment.check_input_owner(facts)?;

        crate::data_library::verify_64bit_sysroot(self.input, "ID reader width")?;
        crate::data_library::verify_core_definition(
            &record.size_of,
            self.input,
            crate::data_library::CoreDefinition {
                paths: &["std::mem::size_of", "core::mem::size_of"],
                unavailable: "Unpinned ID reader size_of authority",
                mismatch: "ID reader SIZE initializer lacks exact core size_of authority",
            },
        )?;
        for definition in [
            &record.definition,
            &record.implementation,
            &record.owner,
            &record.original,
            &record.original_implementation,
            &record.size_argument,
        ] {
            authority::local(facts, definition)?;
        }
        Ok(())
    }
}

fn one<T>(mut items: impl Iterator<Item = T>, description: &str) -> Result<T, String> {
    let item = items
        .next()
        .ok_or_else(|| format!("Missing {description}"))?;
    if items.next().is_some() {
        return Err(format!("Ambiguous {description}"));
    }
    Ok(item)
}

fn names(definition: &Definition) -> Result<Namespace, String> {
    let parts: Namespace = definition.path.split("::").map(str::to_owned).collect();
    if parts.is_empty() || parts.iter().any(String::is_empty) {
        return Err("ID byte width has an incomplete nominal definition path".into());
    }
    Ok(parts)
}

fn terminal(definition: &Definition) -> Result<&str, String> {
    definition
        .path
        .rsplit("::")
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "Missing ID byte width nominal name".into())
}
