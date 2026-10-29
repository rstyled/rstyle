//! Whole-owner occurrence coverage is required for advisory pass-through results.
use super::{Availability, Completion, Coverage, Execution};
use crate::clones::Function;
use rstyle_pdg::{
    ByteSpan, HirRef,
    consumer_facts::{callers, mutations},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize)]
pub(super) struct Report {
    coverage: Coverage,
    execution: Execution,
    expected_functions: usize,
    assessed_functions: usize,
    functions: Vec<FunctionUses>,
    scope: &'static str,
    claim: &'static str,
}

#[derive(Serialize)]
struct FunctionUses {
    function: String,
    availability: Availability<()>,
    coverage: Coverage,
    execution: Execution,
    expected_formals: usize,
    assessed_formals: usize,
    formals: Vec<Formal>,
    boundaries: Vec<Boundary>,
}

#[derive(Serialize)]
struct Formal {
    formal: u32,
    declaration: Option<ByteSpan>,
    mutable: Option<bool>,
    outcome: Outcome,
    occurrences: Vec<Occurrence>,
}

struct FormalAssessment {
    complete: bool,
    rows: Vec<Formal>,
}

struct FormalEvidence<'a> {
    declaration: Option<&'a mutations::UseFormal>,
    occurrences: Vec<Occurrence>,
    complete: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    PassThroughCandidate,
    HasOtherUses,
    Unused,
    Unavailable,
}

#[derive(Serialize)]
struct Occurrence {
    native: HirRef,
    span: ByteSpan,
    role: mutations::UseRole,
    selected_call: Option<SelectedCall>,
    target_resolved: bool,
}

#[derive(Serialize)]
struct SelectedCall {
    function: String,
    span: ByteSpan,
}

#[derive(Serialize)]
struct Boundary {
    span: ByteSpan,
    availability: mutations::BoundaryAvailability,
    reason: String,
}

struct Context<'a> {
    session: &'a callers::Session<'a>,
    selected: BTreeMap<(usize, usize), &'a Function>,
    resolved: BTreeSet<(usize, usize)>,
}

pub(super) fn evaluate(
    records: &BTreeMap<usize, &Function>,
    session: &callers::Session<'_>,
    targets: &[&Function],
    remaining_rows: &mut usize,
) -> Result<Report, String> {
    let ctx = Context::new(records, session, targets);
    let functions = records
        .values()
        .map(|record| ctx.function(record, remaining_rows))
        .collect::<Result<Vec<_>, _>>()?;
    let census = session
        .census()
        .ok_or("Parameter use analysis requires a completed caller scan")?;
    let exhausted = census.execution == mutations::Execution::BudgetExhausted
        || functions
            .iter()
            .any(|function| function.execution == Execution::BudgetExhausted);
    let complete = functions.len() == census.expected_functions
        && functions
            .iter()
            .all(|function| function.coverage == Coverage::Complete);

    Ok(Report {
        coverage: if complete && !exhausted {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        },
        execution: if exhausted {
            Execution::BudgetExhausted
        } else {
            Execution::Completed
        },
        expected_functions: census.expected_functions,
        assessed_functions: records
            .keys()
            .filter(|index| session.observations(**index).is_some())
            .count(),
        functions,
        scope: "owned_u32_formals_in_captured_defining_units",
        claim: "advisory_all_uses_forward_unchanged_to_selected_signature_intents; argument_removal_not_proven",
    })
}

fn reserve_formals(expected: usize, remaining: &mut usize) -> usize {
    let retained = expected.min(super::model::MAX_PLANS).min(*remaining);
    *remaining -= retained;
    retained
}

impl Report {
    pub(super) fn exhausted(&self) -> bool {
        self.execution == Execution::BudgetExhausted
    }
}

impl Completion for Report {
    fn permits_success(&self) -> bool {
        self.execution == Execution::Completed && self.coverage == Coverage::Complete
    }
}

impl<'a> Context<'a> {
    fn new(
        records: &BTreeMap<usize, &Function>,
        session: &'a callers::Session<'a>,
        targets: &[&'a Function],
    ) -> Self {
        let selected = targets
            .iter()
            .flat_map(|target| {
                session
                    .witnesses(target.facts.index)
                    .iter()
                    .map(|witness| ((witness.caller_index, witness.call_index), *target))
            })
            .collect();
        let resolved = records
            .keys()
            .flat_map(|index| {
                session
                    .witnesses(*index)
                    .iter()
                    .map(|witness| (witness.caller_index, witness.call_index))
            })
            .collect();
        Self {
            session,
            selected,
            resolved,
        }
    }

    fn function(
        &self,
        record: &Function,
        remaining_rows: &mut usize,
    ) -> Result<FunctionUses, String> {
        let retained = reserve_formals(record.facts.view().signature.params.len(), remaining_rows);
        let Some(observations) = self.session.observations(record.facts.index) else {
            return Ok(FunctionUses::unavailable(record, retained));
        };
        let formals = self.formals(record, observations, retained)?;

        FunctionUses::observed(record, &observations.uses, formals)
    }

    fn formals(
        &self,
        record: &Function,
        observations: &mutations::Observations,
        retained: usize,
    ) -> Result<FormalAssessment, String> {
        let inventory = &observations.uses;
        let complete = matches!(inventory.availability, Availability::Known(()))
            && inventory.coverage == mutations::Coverage::Complete
            && inventory.execution == mutations::Execution::Completed
            && inventory.formals.len() == record.facts.view().signature.params.len()
            && retained == record.facts.view().signature.params.len();
        let mut occurrences = self.occurrences(record, observations)?;
        let declarations: BTreeMap<_, _> = observations
            .uses
            .formals
            .iter()
            .map(|formal| (formal.ordinal, formal))
            .collect();
        let rows = (0..retained)
            .map(|ordinal| {
                Formal::make(
                    record,
                    ordinal as u32,
                    FormalEvidence {
                        declaration: declarations.get(&(ordinal as u32)).copied(),
                        occurrences: occurrences.remove(&(ordinal as u32)).unwrap_or_default(),
                        complete,
                    },
                )
            })
            .collect::<Result<_, _>>()?;

        Ok(FormalAssessment { complete, rows })
    }

    fn occurrences(
        &self,
        record: &Function,
        observations: &mutations::Observations,
    ) -> Result<BTreeMap<u32, Vec<Occurrence>>, String> {
        let mut occurrences = BTreeMap::<u32, Vec<Occurrence>>::new();
        for original in &observations.uses.occurrences {
            let selected_call = self.selected_call(record, observations, original)?;
            occurrences
                .entry(original.formal)
                .or_default()
                .push(Occurrence {
                    native: original.native.clone(),
                    span: super::regions::absolute(record.facts.view().fn_span, original.span)?,
                    role: original.role,
                    selected_call,
                    target_resolved: match original.role {
                        mutations::UseRole::CallArgument { call_index, .. } => {
                            self.resolved.contains(&(record.facts.index, call_index))
                        }
                        _ => true,
                    },
                });
        }
        Ok(occurrences)
    }

    fn selected_call(
        &self,
        record: &Function,
        observations: &mutations::Observations,
        occurrence: &mutations::UseOccurrence,
    ) -> Result<Option<SelectedCall>, String> {
        let mutations::UseRole::CallArgument {
            call_index,
            argument_index,
            unchanged,
        } = occurrence.role
        else {
            return Ok(None);
        };
        let call = observations
            .calls
            .get(call_index)
            .ok_or("Parameter use references an absent original call")?;
        let argument = call
            .arguments
            .get(argument_index)
            .ok_or("Parameter use references an absent original argument")?;
        if argument.formal != Availability::Known(Some(occurrence.formal))
            || occurrence.span.start < argument.span.start
            || occurrence.span.end > argument.span.end
            || (unchanged
                && argument.unchanged_formal != Availability::Known(Some(occurrence.formal)))
        {
            return Err("Parameter use contradicts its retained original call argument".into());
        }

        let target = self.selected.get(&(record.facts.index, call_index));
        target
            .map(|target| {
                Ok(SelectedCall {
                    function: target.key.to_string(),
                    span: super::regions::absolute(record.facts.view().fn_span, call.span)?,
                })
            })
            .transpose()
    }
}

impl FunctionUses {
    fn observed(
        record: &Function,
        inventory: &mutations::UseInventory,
        assessment: FormalAssessment,
    ) -> Result<Self, String> {
        let FormalAssessment {
            complete,
            rows: formals,
        } = assessment;
        Ok(Self {
            function: record.key.to_string(),
            availability: inventory.availability.clone(),
            coverage: if complete {
                Coverage::Complete
            } else {
                Coverage::Incomplete
            },
            execution: if inventory.execution == mutations::Execution::BudgetExhausted
                || formals.len() < record.facts.view().signature.params.len()
            {
                Execution::BudgetExhausted
            } else {
                Execution::Completed
            },
            expected_formals: record.facts.view().signature.params.len(),
            assessed_formals: inventory.formals.len(),
            formals,
            boundaries: inventory
                .boundaries
                .iter()
                .map(|boundary| {
                    Ok(Boundary {
                        span: super::regions::absolute(record.facts.view().fn_span, boundary.span)?,
                        availability: boundary.availability,
                        reason: boundary.reason.clone(),
                    })
                })
                .collect::<Result<_, String>>()?,
        })
    }

    fn unavailable(record: &Function, retained: usize) -> Self {
        let expected = record.facts.view().signature.params.len();
        Self {
            function: record.key.to_string(),
            availability: Availability::Unknown {
                reason: "Original formal-use inventory was not retained".into(),
            },
            coverage: Coverage::Incomplete,
            execution: if retained < expected {
                Execution::BudgetExhausted
            } else {
                Execution::Completed
            },
            expected_formals: expected,
            assessed_formals: 0,
            formals: (0..retained)
                .map(|formal| Formal {
                    formal: formal as u32,
                    declaration: None,
                    mutable: None,
                    outcome: Outcome::Unavailable,
                    occurrences: vec![],
                })
                .collect(),
            boundaries: Vec::new(),
        }
    }
}

impl Formal {
    fn make(record: &Function, formal: u32, evidence: FormalEvidence<'_>) -> Result<Self, String> {
        let outcome = evidence.outcome();
        let FormalEvidence {
            declaration,
            occurrences,
            ..
        } = evidence;

        Ok(Self {
            formal,
            declaration: declaration
                .map(|formal| super::regions::absolute(record.facts.view().fn_span, formal.span))
                .transpose()?,
            mutable: declaration.map(|formal| formal.mutable),
            outcome,
            occurrences,
        })
    }
}

impl FormalEvidence<'_> {
    fn outcome(&self) -> Outcome {
        if !self.complete || self.declaration.is_none() {
            Outcome::Unavailable
        } else if self.occurrences.is_empty() {
            Outcome::Unused
        } else if self.occurrences.iter().any(Occurrence::proves_other_use) {
            Outcome::HasOtherUses
        } else if self
            .occurrences
            .iter()
            .any(|occurrence| !occurrence.target_resolved)
        {
            Outcome::Unavailable
        } else if self.declaration.is_some_and(|formal| !formal.mutable)
            && self.occurrences.iter().all(|occurrence| {
                matches!(
                    occurrence.role,
                    mutations::UseRole::CallArgument {
                        unchanged: true,
                        ..
                    }
                ) && occurrence.selected_call.is_some()
            })
        {
            Outcome::PassThroughCandidate
        } else {
            Outcome::HasOtherUses
        }
    }
}

impl Occurrence {
    fn proves_other_use(&self) -> bool {
        match self.role {
            mutations::UseRole::CallArgument { unchanged, .. } => {
                !unchanged || (self.target_resolved && self.selected_call.is_none())
            }
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(target_resolved: bool, selected: bool, unchanged: bool) -> Occurrence {
        Occurrence {
            native: HirRef {
                krate: 0,
                owner: 1,
                local: 2,
            },
            span: ByteSpan { start: 10, end: 11 },
            role: mutations::UseRole::CallArgument {
                call_index: 0,
                argument_index: 0,
                unchanged,
            },
            selected_call: selected.then(|| SelectedCall {
                function: "selected-target".into(),
                span: ByteSpan { start: 5, end: 15 },
            }),
            target_resolved,
        }
    }

    fn outcome(occurrences: Vec<Occurrence>, complete: bool, mutable: bool) -> Outcome {
        let declaration = mutations::UseFormal {
            ordinal: 0,
            span: ByteSpan { start: 1, end: 2 },
            native: HirRef {
                krate: 0,
                owner: 1,
                local: 1,
            },
            mutable,
        };
        FormalEvidence {
            declaration: Some(&declaration),
            occurrences,
            complete,
        }
        .outcome()
    }

    #[test]
    fn unavailable_target_cannot_prove_a_selected_or_other_call() {
        assert!(matches!(
            outcome(vec![call(false, false, true)], true, false),
            Outcome::Unavailable
        ));
    }

    #[test]
    fn resolved_targets_distinguish_selected_from_other_calls() {
        assert!(matches!(
            outcome(vec![call(true, true, true)], true, false),
            Outcome::PassThroughCandidate
        ));
        assert!(matches!(
            outcome(vec![call(true, false, true)], true, false),
            Outcome::HasOtherUses
        ));
    }

    #[test]
    fn changed_argument_proves_another_use_even_with_unavailable_target() {
        assert!(matches!(
            outcome(vec![call(false, false, false)], true, false),
            Outcome::HasOtherUses
        ));
        assert!(matches!(
            outcome(vec![call(true, true, false)], true, false),
            Outcome::HasOtherUses
        ));
    }

    #[test]
    fn independent_read_is_a_counterexample_beside_unresolved_forwarding() {
        let mut read = call(true, false, true);
        read.native.local = 3;
        read.role = mutations::UseRole::Read;
        assert!(matches!(
            outcome(vec![call(false, false, true), read], true, false),
            Outcome::HasOtherUses
        ));
    }

    #[test]
    fn incomplete_and_empty_inventories_cannot_become_pass_through() {
        assert!(matches!(outcome(vec![], true, false), Outcome::Unused));
        assert!(matches!(
            outcome(vec![], false, false),
            Outcome::Unavailable
        ));
        assert!(matches!(
            outcome(vec![call(true, true, true)], false, false),
            Outcome::Unavailable
        ));
        assert!(matches!(
            outcome(vec![call(true, true, true)], true, true),
            Outcome::HasOtherUses
        ));
    }

    #[test]
    fn unavailable_formal_rows_share_one_report_allowance_across_owners() {
        let mut remaining = 5;

        assert_eq!(reserve_formals(3, &mut remaining), 3);
        assert_eq!(reserve_formals(3, &mut remaining), 2);
        assert_eq!(reserve_formals(1, &mut remaining), 0);
        assert_eq!(remaining, 0);
    }
}
