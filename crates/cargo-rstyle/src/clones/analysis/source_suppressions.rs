//! Legacy comment syntax applies only to the requested migrated source rules.
use super::{Assessment, Coverage, FactAvailability, Outcome};
use rstyle_core::suppress::{self, Suppression};
use rstyle_pdg::{SourceText, consumer_facts::Availability};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum Report {
    None {
        comment: SourceText,
    },
    Allowed {
        comment: SourceText,
        rules: Vec<String>,
    },
    Invalid {
        comment: SourceText,
        reason: String,
    },
    Unavailable {
        reason: String,
    },
}

pub(super) fn evaluate(comment: &Availability<SourceText>) -> Report {
    let comment = match comment {
        Availability::Known(comment) => comment,
        Availability::Unknown { reason }
        | Availability::Unsupported { reason }
        | Availability::NotApplicable { reason } => return Report::failure(reason.clone()),
    };
    match suppress::parse(comment.as_str()) {
        Suppression::None => Report::None {
            comment: comment.clone(),
        },
        Suppression::Allow(rules) => Report::Allowed {
            comment: comment.clone(),
            rules,
        },
        Suppression::Invalid(reason) => Report::Invalid {
            comment: comment.clone(),
            reason,
        },
    }
}

impl Report {
    pub(super) fn allows(&self, rule: &str) -> bool {
        matches!(rule, "fn_length" | "fn_params" | "fn_spacing")
            && matches!(self, Self::Allowed { rules, .. } if rules.iter().any(|name| name == rule))
    }

    pub(super) fn allows_mutation(&self, rule: &str) -> bool {
        matches!(rule, "mutation_per_target" | "mutated_targets")
            && matches!(self, Self::Allowed { rules, .. } if rules.iter().any(|name| name == rule))
    }

    pub(super) fn allows_density(&self) -> bool {
        matches!(self, Self::Allowed { rules, .. } if rules.iter().any(|rule| rule == "side_effect_density"))
    }

    pub(super) fn complete(&self) -> bool {
        !matches!(self, Self::Unavailable { .. })
    }

    pub(super) fn invalid_assessment(&self) -> Option<Assessment> {
        let Self::Invalid { reason, .. } = self else {
            return None;
        };
        Some(Assessment {
            rule_id: "invalid_suppression",
            outcome: Outcome::Finding,
            coverage: Coverage::Complete,
            availability: FactAvailability::Known,
            actual: Some(1),
            limit: 0,
            reasons: vec![reason.clone()],
        })
    }

    pub(super) fn failure(reason: String) -> Self {
        Self::Unavailable { reason }
    }
}

#[cfg(test)]
mod tests;
