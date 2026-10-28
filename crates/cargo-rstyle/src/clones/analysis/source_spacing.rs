//! Spacing uses complete direct statement evidence and the shared blank-line policy.
use super::{Assessment, Coverage, FactAvailability, Outcome, source_rules};
use rstyle_core::config::FnSpacingConfig;
use rstyle_pdg::consumer_facts::{Availability, LocalFacts};

pub(super) fn evaluate(facts: &LocalFacts<'_>, policy: &FnSpacingConfig) -> Assessment {
    let minimum = policy.min_statements;
    if minimum == 0 {
        return Assessment {
            rule_id: "fn_spacing",
            outcome: Outcome::EvaluatedNoFinding,
            coverage: Coverage::Complete,
            availability: FactAvailability::NotApplicable,
            actual: None,
            limit: 0,
            reasons: vec!["Spacing is disabled by min_statements = 0".into()],
        };
    }

    let limit = minimum.saturating_sub(1);
    let Availability::Known(source) = &facts.source else {
        return source_rules::assess("fn_spacing", &facts.source, limit, |_| 0);
    };
    let mut assessment = source_rules::assess("fn_spacing", &facts.top_statements, limit, |n| *n);
    if let Some(actual) = assessment.actual {
        assessment.outcome = if rstyle_core::spacing::is_finding(source, actual, minimum) {
            Outcome::Finding
        } else {
            Outcome::EvaluatedNoFinding
        };
    }
    assessment
}
