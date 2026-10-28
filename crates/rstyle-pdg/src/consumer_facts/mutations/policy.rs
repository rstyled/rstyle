use super::{super::Availability, Assessment};
use crate::{HirFnFacts, SourceText};

/// Original source inputs checked in the same pass as the direct mutation inventory.
/// Source and comment availability grant no mutation completeness or policy verdict.
#[derive(Debug)]
pub struct PolicyObservation<'a> {
    pub direct: Assessment,
    pub source: Availability<&'a str>,
    pub preceding_comment: Availability<SourceText>,
}

/// Observe direct sites and policy inputs without a second source or syntax inspection.
pub fn inspect_with_policy(facts: &HirFnFacts) -> Result<PolicyObservation<'_>, String> {
    let source = match super::super::owned_source(facts)? {
        Availability::Known(source) => source,
        Availability::Unknown { reason } => {
            return Ok(unavailable(Availability::Unknown { reason }));
        }
        Availability::Unsupported { reason } | Availability::NotApplicable { reason } => {
            return Ok(unavailable(Availability::Unsupported { reason }));
        }
    };
    let (observed, signature_checked) =
        super::observe_original(facts, source, super::Request::default())?;
    let preceding_comment = if signature_checked {
        super::super::preceding_comment::inspect(facts)?
    } else {
        Availability::Unknown {
            reason: "Original signature was not checked for preceding-comment selection".into(),
        }
    };

    Ok(PolicyObservation {
        direct: observed.direct,
        source: Availability::Known(source),
        preceding_comment,
    })
}

fn unavailable(availability: Availability<()>) -> PolicyObservation<'static> {
    PolicyObservation {
        source: unavailable_input(&availability),
        preceding_comment: unavailable_input(&availability),
        direct: super::unavailable(availability),
    }
}

fn unavailable_input<T>(availability: &Availability<()>) -> Availability<T> {
    match availability {
        Availability::Unknown { reason } => Availability::Unknown {
            reason: reason.clone(),
        },
        Availability::Unsupported { reason } => Availability::Unsupported {
            reason: reason.clone(),
        },
        Availability::NotApplicable { reason } => Availability::NotApplicable {
            reason: reason.clone(),
        },
        Availability::Known(()) => Availability::Unknown {
            reason: "Original source authority was not available".into(),
        },
    }
}
