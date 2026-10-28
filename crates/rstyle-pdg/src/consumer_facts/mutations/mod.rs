//! Direct original scalar assignment sites, independent of strict graph admission.
//! Counts describe syntactic sites, never runtime executions, purity or density.
#[cfg(test)]
mod call_tests;
mod calls;
mod model;
mod native;
mod policy;
mod source;
#[cfg(test)]
mod tests;
mod uses;
use super::Availability;
use crate::HirFnFacts;
pub use model::*;
pub use policy::{PolicyObservation, inspect_with_policy};

pub fn inspect(facts: &HirFnFacts) -> Result<Assessment, String> {
    Ok(observe(facts, Request::default())?.direct)
}

/// Observe bounded direct local calls without granting callee effect authority.
pub fn inspect_calls(facts: &HirFnFacts) -> Result<Observations, String> {
    observe(
        facts,
        Request {
            calls: true,
            uses: false,
        },
    )
}

/// Observe calls and original owned-u32 formal uses in the same bounded source scan.
pub fn inspect_calls_with_uses(facts: &HirFnFacts) -> Result<Observations, String> {
    let mut observed = observe(
        facts,
        Request {
            calls: true,
            uses: true,
        },
    )?;
    if matches!(&observed.uses.availability, Availability::Unknown { reason } if reason == "Formal use inventory was not requested")
    {
        observed.uses.availability = match &observed.direct.availability {
            Availability::Known(()) => Availability::Unknown {
                reason: "Original source was not scanned for formal uses".into(),
            },
            availability => availability.clone(),
        };
        observed.uses.execution = observed.direct.execution;
    }
    Ok(observed)
}

#[derive(Clone, Copy, Default)]
pub(super) struct Request {
    pub calls: bool,
    pub uses: bool,
}

fn observe(facts: &HirFnFacts, request: Request) -> Result<Observations, String> {
    let source = match super::owned_source(facts)? {
        Availability::Known(source) => source,
        Availability::Unknown { reason } => {
            return Ok(without_calls(unavailable(Availability::Unknown { reason })));
        }
        Availability::Unsupported { reason } | Availability::NotApplicable { reason } => {
            return Ok(without_calls(unavailable(Availability::Unsupported {
                reason,
            })));
        }
    };
    Ok(observe_original(facts, source, request)?.0)
}

fn observe_original(
    facts: &HirFnFacts,
    source: &str,
    request: Request,
) -> Result<(Observations, bool), String> {
    // Bound parsing before the shared source/signature parser runs.
    if !source::within_budget(&facts.source.bytes) {
        return Ok((
            without_calls(Assessment {
                execution: Execution::BudgetExhausted,
                coverage: Coverage::Incomplete,
                boundaries: vec![Boundary {
                    availability: BoundaryAvailability::Unknown,
                    span: crate::ByteSpan {
                        start: 0,
                        end: facts.source.bytes.len() as u32,
                    },
                    reason: "Original syntax budget exhausted".into(),
                }],
                ..Assessment::empty()
            }),
            false,
        ));
    }
    source::observe_with_signature(facts, source, Assessment::empty(), request)
}

fn unavailable(availability: Availability<()>) -> Assessment {
    Assessment {
        availability,
        coverage: Coverage::Incomplete,
        ..Assessment::empty()
    }
}

fn without_calls(direct: Assessment) -> Observations {
    Observations {
        uses: UseInventory::unrequested(),
        direct,
        calls: vec![],
        signature: Availability::Unknown {
            reason: "Original scalar helper signature was not checked".into(),
        },
    }
}
