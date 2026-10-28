//! Source and signature budgets do not require graph admission.

use super::{Assessment, Coverage, FactAvailability, Outcome};
use rstyle_core::{config::Config, lines::code_lines};
use rstyle_pdg::consumer_facts::{Availability, LocalFacts};

pub(super) fn evaluate(facts: &LocalFacts<'_>, config: &Config) -> Vec<Assessment> {
    vec![
        assess(
            "fn_length",
            &facts.source,
            config.fn_length.max_lines,
            |source| code_lines(source),
        ),
        assess(
            "fn_params",
            &facts.signature,
            config.fn_params.max_params,
            |signature| {
                signature.parameters
                    + usize::from(config.fn_params.count_self && signature.has_self)
            },
        ),
    ]
}

pub(super) fn excluded(config: &Config) -> Vec<Assessment> {
    [
        ("fn_length", config.fn_length.max_lines),
        ("fn_params", config.fn_params.max_params),
    ]
    .into_iter()
    .map(|(rule_id, limit)| Assessment {
        rule_id,
        outcome: Outcome::NotEvaluated,
        coverage: Coverage::Complete,
        availability: FactAvailability::NotApplicable,
        actual: None,
        limit,
        reasons: vec!["Original source path is excluded by the configured workspace policy".into()],
    })
    .collect()
}

fn assess<T>(
    rule_id: &'static str,
    fact: &Availability<T>,
    limit: usize,
    measure: impl FnOnce(&T) -> usize,
) -> Assessment {
    let (availability, reason) = match fact {
        Availability::Known(value) => return measured(rule_id, measure(value), limit),
        Availability::Unknown { reason } => (FactAvailability::Unknown, reason),
        Availability::Unsupported { reason } => (FactAvailability::Unsupported, reason),
        Availability::NotApplicable { reason } => (FactAvailability::NotApplicable, reason),
    };

    Assessment {
        rule_id,
        outcome: Outcome::NotEvaluated,
        coverage: Coverage::Incomplete,
        availability,
        actual: None,
        limit,
        reasons: vec![reason.clone()],
    }
}

fn measured(rule_id: &'static str, actual: usize, limit: usize) -> Assessment {
    Assessment {
        rule_id,
        outcome: if actual > limit {
            Outcome::Finding
        } else {
            Outcome::EvaluatedNoFinding
        },
        coverage: Coverage::Complete,
        availability: FactAvailability::Known,
        actual: Some(actual),
        limit,
        reasons: vec![],
    }
}

#[cfg(test)]
mod tests;
