//! Index borrowed positive values; copy snapshots only for explicit publication.
use super::super::{
    CompilationTarget, Locator,
    baseline_common::{self, Observation, Previous},
    compiler_diagnostics::{Row, Snapshot},
    exact_rules::ReviewSnapshot,
};
use super::{Baseline, Handle};
use std::collections::BTreeMap;

type Key<'a> = (&'a str, &'a Locator, &'a CompilationTarget);

pub(super) fn key(finding: &Snapshot) -> Key<'_> {
    let (locator, context) = finding.identity();
    (&finding.rule_id, locator, &context.compilation_target)
}

pub(super) fn current<'a>(
    rows: &'a [Row<'a>],
    reviews: &'a super::super::compiler_diagnostics::Report,
) -> impl Iterator<Item = Observation<Key<'a>, &'a Snapshot, Handle>> {
    rows.iter().map(|row| Observation {
        key: Handle {
            unit: row.unit,
            diagnostic_index: row.index,
        },
        rule_id: None,
        selected: true,
        review_hidden: reviews.hidden_for(row.unit, row.index),
        finding: row
            .snapshot
            .as_ref()
            .map(|finding| (key(finding), finding))
            .map_err(Clone::clone),
    })
}

pub(super) fn retained(rows: &[Row<'_>], previous: Option<&Baseline>) -> Vec<Previous> {
    let Some(previous) = previous.filter(|baseline| !baseline.entries.is_empty()) else {
        return Vec::new();
    };
    let mut positives = BTreeMap::<_, Vec<_>>::new();
    for finding in rows.iter().filter_map(|row| row.snapshot.as_ref().ok()) {
        positives.entry(key(finding)).or_default().push(finding);
    }

    baseline_common::positive_retained(
        previous
            .entries
            .iter()
            .map(|finding| (key(finding), finding)),
        positives,
    )
}
