//! Join the complete generated source graph and exports before granting a footprint.
use super::{generated_exports, generated_roster, generated_units, *};
use crate::generated_sources::capture as generated;
#[cfg(test)]
mod tests;

/// A source/export receipt grants no callable, ownership or cleanup permission.
pub(crate) struct GeneratedExports {
    _census: Arc<Census>,
    exports: generated_exports::GeneratedExports,
}

impl GeneratedExports {
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.exports.names()
    }

    pub(crate) fn import(&self) -> &Site {
        self.exports.import()
    }
}

/// `modules` must come from the complete source-checked owning producer census.
/// The owning dependency view supplies the compiled-artifact association.
pub(crate) struct GeneratedProducer<'a> {
    pub census: Arc<Census>,
    pub modules: &'a [crate::nominal_boundary::census::Module],
    pub namespaces: &'a UnitCapture,
    pub generated: &'a generated::UnitCapture,
}

pub(crate) fn verify_generated_namespace(
    producer: GeneratedProducer<'_>,
    import: &Import,
) -> Result<GeneratedExports, ProofError> {
    let GeneratedProducer {
        census,
        modules,
        namespaces,
        generated,
    } = producer;

    producer_owner(&census, namespaces, import)?;
    let namespace = import
        .site
        .target
        .path
        .split("::")
        .skip(1)
        .map(str::to_owned)
        .collect::<Vec<_>>();

    let projection = generated_units::verify(&census, modules, &namespace)?;
    let expansion = generated_exports::expansion(&projection, generated, &import.site.target)?;
    generated_roster::verify(&projection, generated, expansion, &import.site.target)?;
    let exports = generated_exports::verify(projection, generated, import)?;

    Ok(GeneratedExports {
        _census: census,
        exports,
    })
}

fn producer_owner(
    census: &Census,
    producer: &UnitCapture,
    import: &Import,
) -> Result<(), ProofError> {
    if producer.identity != capture::IDENTITY
        || census.identity != crate::nominal_boundary::census::IDENTITY
        || import.site.target.local
        || import.site.target.crate_name != census.context.crate_name
        || !import
            .site
            .target
            .path
            .starts_with(&format!("{}::", census.context.crate_name))
    {
        return Err(ProofError::Contradictory(
            "Generated namespace producer identity differs",
        ));
    }

    identity(&import.site.target)?;
    if !producer
        .declarations
        .iter()
        .any(|declaration| same_definition(&declaration.site.owner, &import.site.target))
    {
        return Err(ProofError::Missing(
            "Generated namespace lacks its original producer module owner",
        ));
    }

    if producer.unsupported.iter().any(|finding| {
        same_definition(&finding.definition, &import.site.target)
            || finding.definition.local
                && finding.definition.crate_name == import.site.target.crate_name
                && finding.definition.stable_crate_id == import.site.target.stable_crate_id
                && format!(
                    "{}::{}",
                    finding.definition.crate_name, finding.definition.path
                )
                .starts_with(&format!("{}::", import.site.target.path))
    }) {
        return Err(ProofError::Unsupported(
            "Generated namespace retains unsupported producer evidence",
        ));
    }

    Ok(())
}
