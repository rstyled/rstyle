//! Index borrowed positive values; copy snapshots only for explicit publication.
use super::super::{
    CompilationTarget, Locator,
    baseline_common::{self, Observation, Previous},
    compiler_diagnostics::{Row, Snapshot},
    exact_rules::ReviewSnapshot,
};
use super::{Baseline, Handle};

type Key<'a> = (&'a str, &'a Locator, &'a CompilationTarget);

pub(super) fn key(finding: &Snapshot) -> Key<'_> {
    let (locator, context) = finding.identity();
    (&finding.rule_id, locator, &context.compilation_target)
}

pub(super) fn current<'a>(
    rows: &'a [Row<'a>],
    reviews: &'a super::super::compiler_diagnostics::Report,
    selection: Option<&'a crate::clones::analysis::changed_scope::Selection>,
) -> impl Iterator<Item = Observation<Key<'a>, &'a Snapshot, Handle>> {
    rows.iter().map(move |row| Observation {
        key: Handle {
            unit: row.unit,
            diagnostic_index: row.index,
        },
        rule_id: None,
        selected: selection.is_none_or(|selection| selection.contains(row.path)),
        review_hidden: reviews.hidden_for(row.unit, row.index),
        finding: row
            .snapshot
            .as_ref()
            .map(|finding| (key(finding), finding))
            .map_err(Clone::clone),
    })
}

pub(super) fn retained(
    advisory: &crate::clones::advisory::Report,
    ctx: &crate::clones::report::Context<'_>,
    previous: Option<&Baseline>,
) -> Vec<Previous> {
    let Some(previous) = previous.filter(|baseline| !baseline.entries.is_empty()) else {
        return Vec::new();
    };
    let owners = super::super::compiler_diagnostics::retained::observations(advisory, ctx);
    baseline_common::retained(
        previous.entries.iter().map(|finding| {
            let (locator, context) = finding.identity();
            (locator, context, finding)
        }),
        super::super::owner_population::Population::new(
            &owners.rows,
            owners.unavailable_scope,
            owners.truncated,
        ),
        super::super::owner_population::Domain::Compiler,
        super::super::numeric_workflow::Retained::assess,
    )
}
