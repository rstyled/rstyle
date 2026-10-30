//! Borrow completed proposal snapshots; membership keys contain references, never graph copies.
use super::super::{
    baseline_common::{self, Observation, Previous},
    compiler_patterns::{self, observed::Row},
};
use super::{Baseline, Handle};
use std::collections::BTreeMap;

pub(super) fn current<'a>(
    rows: &'a [Row<'a>],
    reviews: &'a compiler_patterns::Report,
    selected: &'a BTreeMap<crate::clones::identity::UnitId, &[usize]>,
) -> impl Iterator<
    Item = Observation<
        compiler_patterns::Key<'a>,
        &'a compiler_patterns::observed::Snapshot,
        Handle,
    >,
> {
    rows.iter().map(|row| Observation {
        key: Handle {
            unit: row.unit,
            candidate_index: row.index,
        },
        rule_id: None,
        selected: selected
            .get(&row.unit)
            .is_none_or(|indices| indices.binary_search(&row.index).is_ok()),
        review_hidden: reviews.hidden_for(row.unit, row.index),
        finding: row
            .snapshot
            .as_ref()
            .map(|finding| (finding.baseline_key(), finding))
            .map_err(Clone::clone),
    })
}

pub(super) fn retained(rows: &[Row<'_>], previous: Option<&Baseline>) -> Vec<Previous> {
    let Some(previous) = previous.filter(|baseline| !baseline.entries.is_empty()) else {
        return Vec::new();
    };
    let mut positives = BTreeMap::<_, Vec<_>>::new();
    for finding in rows.iter().filter_map(|row| row.snapshot.as_ref().ok()) {
        positives
            .entry(finding.baseline_key())
            .or_default()
            .push(finding);
    }

    baseline_common::positive_retained(
        previous
            .entries
            .iter()
            .map(|finding| (finding.baseline_key(), finding)),
        positives,
    )
}
