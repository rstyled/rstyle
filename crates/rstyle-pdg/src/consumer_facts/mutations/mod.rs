//! Direct original scalar assignment sites, independent of strict graph admission.
//! Counts describe syntactic sites, never runtime executions, purity or density.
#[cfg(test)]
mod call_tests;
mod calls;
mod model;
mod native;
mod source;
#[cfg(test)]
mod tests;
use super::Availability;
use crate::HirFnFacts;
pub use model::*;

pub fn inspect(facts: &HirFnFacts) -> Result<Assessment, String> {
    Ok(observe(facts, false)?.direct)
}

/// Observe bounded direct local calls without granting callee effect authority.
pub fn inspect_calls(facts: &HirFnFacts) -> Result<Observations, String> {
    observe(facts, true)
}

fn observe(facts: &HirFnFacts, calls: bool) -> Result<Observations, String> {
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
    // Bound parsing before the shared source/signature parser runs.
    if !source::within_budget(&facts.source.bytes) {
        return Ok(without_calls(Assessment {
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
        }));
    }
    if calls {
        source::observe(facts, source, Assessment::empty(), true)
    } else {
        source::inspect(facts, source, Assessment::empty()).map(without_calls)
    }
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
        direct,
        calls: vec![],
        signature: Availability::Unknown {
            reason: "Original scalar helper signature was not checked".into(),
        },
    }
}
