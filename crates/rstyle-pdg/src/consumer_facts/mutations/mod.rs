//! Direct original scalar assignment sites, independent of strict graph admission.
//! Counts describe syntactic sites, never runtime executions, purity or density.
mod model;
mod native;
mod source;
#[cfg(test)]
mod tests;
use super::Availability;
use crate::HirFnFacts;
pub use model::*;

pub fn inspect(facts: &HirFnFacts) -> Result<Assessment, String> {
    let source = match super::owned_source(facts)? {
        Availability::Known(source) => source,
        Availability::Unknown { reason } => {
            return Ok(unavailable(Availability::Unknown { reason }));
        }
        Availability::Unsupported { reason } | Availability::NotApplicable { reason } => {
            return Ok(unavailable(Availability::Unsupported { reason }));
        }
    };
    // Bound parsing before the shared source/signature parser runs.
    if !source::within_budget(&facts.source.bytes) {
        return Ok(Assessment {
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
        });
    }
    source::inspect(facts, source, Assessment::empty())
}

fn unavailable(availability: Availability<()>) -> Assessment {
    Assessment {
        availability,
        coverage: Coverage::Incomplete,
        ..Assessment::empty()
    }
}
