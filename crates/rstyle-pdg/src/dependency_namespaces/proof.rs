//! Bounded original-source closure for anonymous trait exports.
//! This receipt proves an export footprint, never a Cargo artifact association,
//! trait implementation, callable effect, or admission decision.
mod consumer;
mod generated_exports;
mod generated_namespace;
mod generated_roster;
mod generated_units;
mod source;
pub use consumer::verify_consumer;
pub(crate) use consumer::verify_site;
pub(crate) use generated_namespace::{
    GeneratedExports, GeneratedProducer, verify_generated_namespace,
};
#[cfg(test)]
mod tests;

use super::capture::{self, Binding, Import, RouteKind, Site, UnitCapture};
use crate::{generated_sources::capture::Definition, nominal_boundary::census::Census};
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
pub enum ProofError {
    Missing(&'static str),
    Unsupported(&'static str),
    Contradictory(&'static str),
}

impl std::fmt::Display for ProofError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(message) | Self::Unsupported(message) | Self::Contradictory(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for ProofError {}

/// Nonserializable source evidence. The caller must separately bind the exact
/// consumer and producer through a checked direct compiled-artifact association.
pub struct AnonymousPrelude {
    census: Arc<Census>,
    module: Definition,
    traits: [Definition; 2],
    import: Site,
}

impl AnonymousPrelude {
    pub fn census(&self) -> &Arc<Census> {
        &self.census
    }

    pub fn module(&self) -> &Definition {
        &self.module
    }

    pub fn traits(&self) -> &[Definition; 2] {
        &self.traits
    }

    pub fn import(&self) -> &Site {
        &self.import
    }
}

pub fn verify_anonymous_prelude(
    census: Arc<Census>,
    producer: &UnitCapture,
    consumer: &Import,
) -> Result<AnonymousPrelude, ProofError> {
    inputs(&census, producer, consumer)?;
    let source = source::Prelude::original(&census)?;
    let sites = source.declarations(producer)?;
    exports(consumer, &sites)?;
    unsupported(producer, consumer, &sites)?;

    Ok(AnonymousPrelude {
        census,
        module: consumer.site.target.clone(),
        traits: [sites[0].target.clone(), sites[1].target.clone()],
        import: consumer.site.clone(),
    })
}

fn inputs(census: &Census, producer: &UnitCapture, consumer: &Import) -> Result<(), ProofError> {
    if producer.identity != capture::IDENTITY
        || census.identity != crate::nominal_boundary::census::IDENTITY
    {
        return Err(ProofError::Contradictory(
            "Namespace capture or census identity differs",
        ));
    }
    census
        .context
        .validate()
        .map_err(|_| ProofError::Contradictory("Invalid producer package source"))?;
    if census.files.len() > crate::nominal_boundary::census::MAX_FILES
        || census
            .files
            .iter()
            .try_fold(0usize, |size, file| size.checked_add(file.bytes.len()))
            .is_none_or(|size| size > crate::nominal_boundary::census::MAX_BYTES)
    {
        return Err(ProofError::Unsupported(
            "Producer source exceeds finite census budget",
        ));
    }

    let site = &consumer.site;
    if site.binding != Binding::Glob || site.target_kind != "Mod" {
        return Err(ProofError::Unsupported(
            "Only a direct module glob is supported",
        ));
    }
    if site.target.local
        || site.target.path != format!("{}::prelude", census.context.crate_name)
        || site.target.crate_name != census.context.crate_name
    {
        return Err(ProofError::Contradictory(
            "Foreign prelude owner differs from producer",
        ));
    }
    identity(&site.target)?;
    if consumer.exports.len() != 2 {
        return Err(ProofError::Contradictory(
            "Anonymous prelude requires exactly two compiler exports",
        ));
    }
    // Neither value of raw `complete` grants source authority. The exact closed
    // original module below independently excludes hidden/ambiguous exports.
    Ok(())
}

fn identity(definition: &Definition) -> Result<(), ProofError> {
    if definition.path.is_empty()
        || definition.definition_identity.is_empty()
        || definition.crate_name.is_empty()
        || definition.stable_crate_id.is_empty()
    {
        return Err(ProofError::Missing(
            "Incomplete compiler definition identity",
        ));
    }
    Ok(())
}

fn same_definition(local: &Definition, foreign: &Definition) -> bool {
    local.local
        && !foreign.local
        && format!("{}::{}", local.crate_name, local.path) == foreign.path
        && local.definition_identity == foreign.definition_identity
        && local.crate_name == foreign.crate_name
        && local.stable_crate_id == foreign.stable_crate_id
}

fn exports(consumer: &Import, sites: &[&Site; 2]) -> Result<(), ProofError> {
    for (export, site) in consumer.exports.iter().zip(sites) {
        let [route] = export.route.as_slice() else {
            return Err(ProofError::Unsupported(
                "Anonymous exports require one direct original route",
            ));
        };
        if export.binding != Binding::Anonymous
            || !export.public
            || export.target_kind != "Trait"
            || !matches!(route.kind, RouteKind::Single)
            || !same_definition(&site.definition, &route.declaration)
            || !same_definition(&site.target, &export.target)
            || !same_definition(&site.owner, &consumer.site.target)
        {
            return Err(ProofError::Contradictory(
                "Ordered anonymous export identity or route differs",
            ));
        }
    }
    Ok(())
}

fn unsupported(
    producer: &UnitCapture,
    consumer: &Import,
    sites: &[&Site; 2],
) -> Result<(), ProofError> {
    for finding in &producer.unsupported {
        let selected = same_definition(&finding.definition, &consumer.site.target)
            || (finding.definition.local
                && finding.definition.crate_name == consumer.site.target.crate_name
                && finding.definition.stable_crate_id == consumer.site.target.stable_crate_id
                && format!(
                    "{}::{}",
                    finding.definition.crate_name, finding.definition.path
                )
                .starts_with(&format!("{}::", consumer.site.target.path)))
            || sites.iter().any(|site| {
                finding.definition == site.definition || finding.definition == site.target
            });
        if selected {
            return Err(ProofError::Unsupported(
                "Selected producer namespace retains unsupported evidence",
            ));
        }
    }
    Ok(())
}
