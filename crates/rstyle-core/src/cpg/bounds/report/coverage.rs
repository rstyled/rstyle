//! Counts over the full input roster, before report selection. These are separate measures:
//! a proven zero-loop owner, a proven scan and a library contract are different information.
use super::Evidence;
use crate::cpg::records::{Measured, Record};
use crate::runtime_bounds::{Bound, Outcome, compose::Composed};
use serde::Serialize;
use std::collections::BTreeMap;

type Counts = BTreeMap<&'static str, usize>;

/// Distinct counts of captured inputs, proof outcomes, loop work and library completeness.
#[derive(Serialize)]
pub struct Coverage {
    roster: Counts,
    capture: Counts,
    local: Counts,
    composed: Counts,
    proven_local_loops: Counts,
    proven_composed_loops: Counts,
    library_steps: Counts,
}

impl Coverage {
    /// Counts only the supplied roster and evidence, before presentation selection. A caller
    /// must state which owners it supplied; absent owners cannot be inferred from these counts.
    pub fn new<'a>(
        records: impl Iterator<Item = &'a Record>,
        evidence: impl Iterator<Item = &'a Evidence> + Clone,
    ) -> Self {
        Self::of(records.map(roster), evidence)
    }

    /// As [`Coverage::new`], over each supplied owner's roster entry as [`roster`] names it.
    pub(crate) fn of<'a>(
        roster: impl Iterator<Item = &'static str>,
        evidence: impl Iterator<Item = &'a Evidence> + Clone,
    ) -> Self {
        Self {
            roster: counts(roster),
            capture: counts(evidence.clone().map(|item| item.capture_status)),
            local: counts(evidence.clone().map(local)),
            composed: counts(evidence.clone().map(composed)),
            proven_local_loops: counts(evidence.clone().filter_map(local_loops)),
            proven_composed_loops: counts(evidence.clone().filter_map(composed_loops)),
            library_steps: counts(evidence.map(library)),
        }
    }

    /// Whether all supplied owners have measured input and all supplied bounds are captured.
    pub fn input_complete(&self) -> bool {
        self.roster.get("unavailable").copied().unwrap_or(0) == 0
            && self.capture.keys().all(|status| *status == "captured")
    }
}

fn counts(items: impl Iterator<Item = &'static str>) -> Counts {
    let mut counts = Counts::new();
    for item in items {
        *counts.entry(item).or_default() += 1;
    }
    counts
}

pub(super) fn roster(record: &Record) -> &'static str {
    match record.measured {
        Measured::Known(_) => "authored_functions",
        Measured::Body(_) => "non_function_bodies",
        Measured::Generated(_) => "generated",
        Measured::Unavailable(_) => "unavailable",
    }
}

fn local(evidence: &Evidence) -> &'static str {
    match evidence.result {
        Some(Outcome::Proven(_)) => "proven",
        Some(Outcome::Partial(_)) => "partial",
        Some(Outcome::Unknown { .. }) => "unknown",
        Some(Outcome::BudgetExceeded { .. }) => "budget_exceeded",
        None => "not_available",
    }
}

fn composed(evidence: &Evidence) -> &'static str {
    match evidence.composed {
        Some(Composed::Proven(_)) => "proven",
        Some(Composed::Partial(_)) => "partial",
        Some(Composed::Unknown { .. }) => "unknown",
        Some(Composed::BudgetExceeded { .. }) => "budget_exceeded",
        None => "not_available",
    }
}

fn local_loops(evidence: &Evidence) -> Option<&'static str> {
    let Some(Outcome::Proven(assessment)) = &evidence.result else {
        return None;
    };
    Some(if assessment.loops.is_empty() {
        "without_modeled_loops"
    } else {
        loop_work(&assessment.loop_body_entries)
    })
}

fn composed_loops(evidence: &Evidence) -> Option<&'static str> {
    let Some(Composed::Proven(composition)) = &evidence.composed else {
        return None;
    };
    Some(
        if composition.loop_body_entries.evaluate(&|_| None) == Some(0) {
            "zero_loop_work"
        } else {
            loop_work(&composition.loop_body_entries)
        },
    )
}

fn loop_work(bound: &Bound) -> &'static str {
    if bound.order() == Default::default() {
        "constant_loop_work"
    } else {
        "input_dependent_loop_work"
    }
}

fn library(evidence: &Evidence) -> &'static str {
    match &evidence.composed {
        Some(Composed::Proven(composition) | Composed::Partial(composition)) => {
            if composition.library_steps.is_complete() {
                "complete"
            } else {
                "opaque"
            }
        }
        _ => "not_available",
    }
}
