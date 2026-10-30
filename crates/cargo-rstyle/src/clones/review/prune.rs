//! Prune only independently completed negative assessments of retained reviews.
use super::{Locator, ReviewStore, exact_rules, local, status};
use crate::clones::analysis::{Coverage, Execution};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub(in crate::clones) struct Selection {
    pub family: status::Family,
    pub entry_index: usize,
}

#[derive(Debug, Serialize)]
pub(in crate::clones) struct Report {
    pub schema: &'static str,
    pub dry_run: bool,
    pub eligible_entries: usize,
    pub removed_entries: usize,
    pub retained_entries: usize,
    pub selections: Vec<Selection>,
}

impl Report {
    pub fn render(&self) -> String {
        format!(
            "\nRetained review pruning{}: {} eligible, {} removed; {} retained entries.",
            if self.dry_run { " (dry run)" } else { "" },
            self.eligible_entries,
            self.removed_entries,
            self.retained_entries
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Ordinal {
    Helper(usize),
    Source(usize),
    Parameter(usize),
    Length(usize),
    Spacing(usize),
    Mutation(usize),
    Density(usize),
    CompilerPattern(usize),
    CompilerDiagnostic(usize),
    Guard(usize),
}

pub(super) fn apply(
    store: &mut ReviewStore,
    status: &status::Report,
    dry_run: bool,
) -> Result<Report, String> {
    validate_summary(store, status)?;
    let plan = selection_plan(store, status)?;

    if !dry_run {
        retain_structural(store, &plan.states);
        retain_numeric(store, &plan.states);
    }
    Ok(Report {
        schema: "rstyle.review-prune/1",
        dry_run,
        eligible_entries: plan.selections.len(),
        removed_entries: if dry_run { 0 } else { plan.selections.len() },
        retained_entries: store.retained_entries(),
        selections: plan.selections,
    })
}

struct SelectionPlan {
    states: BTreeMap<Ordinal, bool>,
    selections: Vec<Selection>,
}

fn retain_structural(store: &mut ReviewStore, states: &BTreeMap<Ordinal, bool>) {
    retain(&mut store.entries, states, Ordinal::Helper);
    retain(&mut store.source_entries, states, Ordinal::Source);
    retain(&mut store.guard_entries, states, Ordinal::Guard);
}

fn selection_plan(store: &ReviewStore, status: &status::Report) -> Result<SelectionPlan, String> {
    let mut states = BTreeMap::new();
    let mut selections = Vec::new();
    for record in &status.records {
        let ordinal = validate_record(store, record)?;
        let negative = record.state == status::State::NoFinding;
        if states.insert(ordinal, negative).is_some() {
            return Err("Duplicate retained review status entry".into());
        }
        if negative {
            selections.push(Selection {
                family: record.family,
                entry_index: record.entry_index,
            });
        }
    }

    Ok(SelectionPlan { states, selections })
}

fn retain_numeric(store: &mut ReviewStore, states: &BTreeMap<Ordinal, bool>) {
    retain_source_rules(store, states);
    retain_effects_and_patterns(store, states);
}

fn retain_effects_and_patterns(store: &mut ReviewStore, states: &BTreeMap<Ordinal, bool>) {
    retain_compiler(store, states);
    retain(&mut store.mutation_entries, states, Ordinal::Mutation);
    retain(&mut store.density_entries, states, Ordinal::Density);
}

fn retain_compiler(store: &mut ReviewStore, states: &BTreeMap<Ordinal, bool>) {
    retain(
        &mut store.compiler_pattern_entries,
        states,
        Ordinal::CompilerPattern,
    );
    retain(
        &mut store.compiler_diagnostic_entries,
        states,
        Ordinal::CompilerDiagnostic,
    );
}

fn retain_source_rules(store: &mut ReviewStore, states: &BTreeMap<Ordinal, bool>) {
    retain(&mut store.parameter_entries, states, Ordinal::Parameter);
    retain(&mut store.length_entries, states, Ordinal::Length);
    retain(&mut store.spacing_entries, states, Ordinal::Spacing);
}

fn retain<T>(
    entries: &mut Vec<T>,
    states: &BTreeMap<Ordinal, bool>,
    ordinal: fn(usize) -> Ordinal,
) {
    let mut index = 0;
    entries.retain(|_| {
        let keep = states.get(&ordinal(index)) != Some(&true);
        index += 1;
        keep
    });
}

fn validate_summary(store: &ReviewStore, report: &status::Report) -> Result<(), String> {
    let total = store.retained_entries();
    let unassessed = report
        .records
        .iter()
        .filter(|record| {
            matches!(
                record.state,
                status::State::NotObserved
                    | status::State::NotRequested
                    | status::State::Unavailable
            )
        })
        .count();
    let coverage = if unassessed == 0 {
        Coverage::Complete
    } else {
        Coverage::Incomplete
    };

    if report.schema != "rstyle.review-status/1"
        || report.execution != Execution::Completed
        || report.records.len() != total
        || report.retained_entries != total
        || report.unassessed_entries != unassessed
        || report.assessed_entries != total - unassessed
        || report.coverage != coverage
    {
        return Err(
            "Pruning requires a complete, consistent retained-entry status inventory".into(),
        );
    }
    Ok(())
}

fn validate_record(store: &ReviewStore, record: &status::Record) -> Result<Ordinal, String> {
    match record.family {
        status::Family::Helper => {
            let entry = store
                .entries
                .get(record.entry_index)
                .ok_or_else(invalid_record)?;
            validate_identity(
                record,
                &entry.reason,
                entry
                    .finding
                    .members
                    .iter()
                    .map(|member| &member.key.locator),
            )?;
            Ok(Ordinal::Helper(record.entry_index))
        }
        status::Family::Parameter => {
            validate_numeric_record(&store.parameter_entries, record, Ordinal::Parameter)
        }
        status::Family::Length => {
            validate_numeric_record(&store.length_entries, record, Ordinal::Length)
        }
        status::Family::Spacing => {
            validate_numeric_record(&store.spacing_entries, record, Ordinal::Spacing)
        }
        status::Family::GuardClause => {
            if record.state == status::State::NoFinding {
                return Err("Guard review discovery does not establish completed negatives".into());
            }
            validate_numeric_record(&store.guard_entries, record, Ordinal::Guard)
        }
        status::Family::MutationPerTarget | status::Family::MutatedTargets => {
            validate_mutation_record(store, record)
        }
        status::Family::SideEffectDensity => {
            validate_numeric_record(&store.density_entries, record, Ordinal::Density)
        }
        status::Family::CompilerDiagnostic | status::Family::CompilerPattern => {
            validate_compiler_record(store, record)
        }
        _ => validate_source_record(store, record),
    }
}

fn validate_compiler_record(
    store: &ReviewStore,
    record: &status::Record,
) -> Result<Ordinal, String> {
    match record.family {
        status::Family::CompilerDiagnostic => {
            let entry = store
                .compiler_diagnostic_entries
                .get(record.entry_index)
                .ok_or_else(invalid_record)?;
            validate_identity(
                record,
                &entry.reason,
                std::iter::once(&entry.finding.owner.locator),
            )?;
            Ok(Ordinal::CompilerDiagnostic(record.entry_index))
        }
        status::Family::CompilerPattern => {
            let entry = store
                .compiler_pattern_entries
                .get(record.entry_index)
                .ok_or_else(invalid_record)?;
            validate_identity(record, &entry.reason, entry.finding.locators())?;
            if record.state == status::State::NoFinding {
                return Err("Bounded compiler proposal discovery does not establish negative review assessments".into());
            }
            Ok(Ordinal::CompilerPattern(record.entry_index))
        }
        _ => Err(invalid_record()),
    }
}

fn validate_mutation_record(
    store: &ReviewStore,
    record: &status::Record,
) -> Result<Ordinal, String> {
    let entry = store
        .mutation_entries
        .get(record.entry_index)
        .ok_or_else(invalid_record)?;
    let expected = match entry.finding.policy {
        super::mutation_rules::Policy::MutationPerTarget { .. } => {
            status::Family::MutationPerTarget
        }
        super::mutation_rules::Policy::MutatedTargets { .. } => status::Family::MutatedTargets,
    };
    if record.family != expected {
        return Err(invalid_record());
    }
    validate_numeric_record(&store.mutation_entries, record, Ordinal::Mutation)
}

fn validate_numeric_record<S: exact_rules::ReviewSnapshot>(
    entries: &[exact_rules::Entry<S>],
    record: &status::Record,
    ordinal: fn(usize) -> Ordinal,
) -> Result<Ordinal, String> {
    let entry = entries.get(record.entry_index).ok_or_else(invalid_record)?;
    validate_identity(
        record,
        &entry.reason,
        std::iter::once(entry.finding.identity().0),
    )?;
    Ok(ordinal(record.entry_index))
}

fn validate_source_record(store: &ReviewStore, record: &status::Record) -> Result<Ordinal, String> {
    let entry = store
        .source_entries
        .get(record.entry_index)
        .ok_or_else(invalid_record)?;
    let expected = match entry.finding.proposal {
        local::Proposal::EnumDispatch { .. } => status::Family::EnumDispatch,
        local::Proposal::OrderedCapability { .. } => status::Family::CapabilitySet,
    };
    if record.family != expected {
        return Err(invalid_record());
    }

    validate_identity(
        record,
        &entry.reason,
        entry.finding.members.iter().map(|member| &member.locator),
    )?;
    Ok(Ordinal::Source(record.entry_index))
}

fn validate_identity<'a>(
    record: &status::Record,
    reason: &str,
    members: impl Iterator<Item = &'a Locator>,
) -> Result<(), String> {
    if record.reason != reason || !record.members.iter().eq(members) {
        return Err(invalid_record());
    }
    Ok(())
}

fn invalid_record() -> String {
    "Retained review status does not identify the original stored entry".into()
}

#[cfg(test)]
mod tests;
