//! Region intersection is coordination evidence, never rewrite correctness.
use super::model::*;
use rstyle_pdg::ByteSpan;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn analyze(plans: &[Plan<'_>], scope_complete: bool) -> Analysis {
    let bounded = &plans[..plans.len().min(MAX_PLANS)];
    let validation = validate_plans(bounded);
    let pairs = assess_pairs(bounded, &validation.statuses);
    let exhausted = plans.len() > MAX_PLANS || validation.exhausted || pairs.exhausted;
    let known = validation
        .statuses
        .values()
        .all(|status| *status == Status::Known);

    Analysis {
        execution: if exhausted {
            Execution::BudgetExhausted
        } else {
            Execution::Completed
        },
        coverage_complete: scope_complete && known && !exhausted,
        expected_comparisons: scope_complete.then(|| pair_count(plans.len())),
        assessed_comparisons: pairs.relations.len(),
        region_comparisons: pairs.region_comparisons,
        snapshot_comparison_bytes: validation.comparison_bytes,
        conflicting_snapshot_plans: validation.plans_with(Status::Conflicting),
        unavailable_plans: validation.plans_with(Status::Unavailable),
        overlap_groups: groups(&pairs.relations),
        relations: pairs.relations,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Known,
    Conflicting,
    Unavailable,
}

struct Snapshot<'a> {
    bytes: &'a str,
    status: Status,
}

#[derive(Default)]
struct SnapshotLedger<'a> {
    snapshots: BTreeMap<&'a str, Snapshot<'a>>,
    comparison_bytes: usize,
    exhausted: bool,
}

struct SnapshotCheck {
    status: Status,
    comparison_bytes: usize,
    exhausted: bool,
}

impl<'a> SnapshotLedger<'a> {
    fn observe(&mut self, region: &Region<'a>) {
        let original = self.snapshots.get(region.resource);
        let checked = check_snapshot(
            original,
            region.snapshot,
            MAX_SNAPSHOT_BYTES - self.comparison_bytes,
        );
        let bytes = original.map_or(region.snapshot, |snapshot| snapshot.bytes);

        self.snapshots.insert(
            region.resource,
            Snapshot {
                bytes,
                status: checked.status,
            },
        );
        self.comparison_bytes += checked.comparison_bytes;
        self.exhausted |= checked.exhausted;
    }
}

struct Validation {
    statuses: BTreeMap<usize, Status>,
    comparison_bytes: usize,
    exhausted: bool,
}

impl Validation {
    fn plans_with(&self, status: Status) -> Vec<usize> {
        self.statuses
            .iter()
            .filter_map(|(id, observed)| (*observed == status).then_some(*id))
            .collect()
    }
}

fn validate_plans(plans: &[Plan<'_>]) -> Validation {
    let mut ledger = SnapshotLedger::default();
    for region in plans.iter().flat_map(|plan| plan.regions.iter().take(2)) {
        ledger.observe(region);
    }

    Validation {
        statuses: plans
            .iter()
            .map(|plan| (plan.id, plan_status(plan, &ledger.snapshots)))
            .collect(),
        comparison_bytes: ledger.comparison_bytes,
        exhausted: ledger.exhausted,
    }
}

fn check_snapshot(original: Option<&Snapshot<'_>>, bytes: &str, remaining: usize) -> SnapshotCheck {
    let Some(original) = original else {
        return snapshot_check(Status::Known, 0, false);
    };
    if original.status != Status::Known || std::ptr::eq(original.bytes, bytes) {
        return snapshot_check(original.status, 0, false);
    }
    if original.bytes.len() != bytes.len() {
        return snapshot_check(Status::Conflicting, 0, false);
    }
    if bytes.len() > remaining {
        return snapshot_check(Status::Unavailable, 0, true);
    }

    let status = if original.bytes == bytes {
        Status::Known
    } else {
        Status::Conflicting
    };
    snapshot_check(status, bytes.len(), false)
}

fn snapshot_check(status: Status, comparison_bytes: usize, exhausted: bool) -> SnapshotCheck {
    SnapshotCheck {
        status,
        comparison_bytes,
        exhausted,
    }
}

fn plan_status(plan: &Plan<'_>, snapshots: &BTreeMap<&str, Snapshot<'_>>) -> Status {
    if plan.regions.len() != 2 {
        return Status::Unavailable;
    }
    let collected = [
        snapshots[plan.regions[0].resource].status,
        snapshots[plan.regions[1].resource].status,
    ];
    if collected.contains(&Status::Conflicting) {
        Status::Conflicting
    } else if collected.contains(&Status::Unavailable) {
        Status::Unavailable
    } else {
        Status::Known
    }
}

fn pair_count(count: usize) -> usize {
    count.saturating_mul(count.saturating_sub(1)) / 2
}

#[derive(Default)]
struct PairAssessment {
    relations: Vec<Pair>,
    region_comparisons: usize,
    exhausted: bool,
}

struct Compared {
    relation: Option<Relation>,
    work: usize,
}

impl PairAssessment {
    fn record(self, first: usize, second: usize, compared: Compared) -> Self {
        let exhausted = compared.relation.is_none();
        let pair = compared.relation.map(|relation| Pair {
            first,
            second,
            relation,
        });
        let mut relations = self.relations;
        relations.extend(pair);

        Self {
            relations,
            region_comparisons: self.region_comparisons + compared.work,
            exhausted: self.exhausted || exhausted,
        }
    }

    fn exhausted(self) -> Self {
        Self {
            exhausted: true,
            ..self
        }
    }
}

fn assess_pairs(plans: &[Plan<'_>], statuses: &BTreeMap<usize, Status>) -> PairAssessment {
    let mut result = PairAssessment::default();
    for (first_index, first) in plans.iter().enumerate() {
        for second in &plans[first_index + 1..] {
            if result.relations.len() == MAX_COMPARISONS {
                return result.exhausted();
            }
            let compared = compare(
                first,
                second,
                statuses,
                MAX_COMPARISONS - result.region_comparisons,
            );
            result = result.record(first.id, second.id, compared);
            if result.exhausted {
                return result;
            }
        }
    }
    result
}

fn compare(
    first: &Plan<'_>,
    second: &Plan<'_>,
    statuses: &BTreeMap<usize, Status>,
    remaining: usize,
) -> Compared {
    let states = [statuses[&first.id], statuses[&second.id]];
    if states.contains(&Status::Conflicting) {
        return Compared {
            relation: Some(Relation::ConflictingSnapshots),
            work: 0,
        };
    }
    if states.contains(&Status::Unavailable) {
        return Compared {
            relation: Some(Relation::Unavailable),
            work: 0,
        };
    }
    let work = first.regions.len() * second.regions.len();
    if work > remaining {
        return Compared {
            relation: None,
            work: remaining,
        };
    }

    let witnesses = witnesses(first, second);
    let relation = if witnesses.is_empty() {
        Relation::DisjointKnownRegions
    } else {
        Relation::OverlapWitness { witnesses }
    };
    Compared {
        relation: Some(relation),
        work,
    }
}

fn witnesses(first: &Plan<'_>, second: &Plan<'_>) -> Vec<Witness> {
    first
        .regions
        .iter()
        .enumerate()
        .flat_map(|(first_region, left)| {
            second
                .regions
                .iter()
                .enumerate()
                .filter_map(move |(second_region, right)| {
                    if left.resource != right.resource {
                        return None;
                    }
                    Some(Witness {
                        first_region,
                        second_region,
                        intersection: intersection(left.span, right.span)?,
                    })
                })
        })
        .collect()
}

fn intersection(first: ByteSpan, second: ByteSpan) -> Option<ByteSpan> {
    let start = first.start.max(second.start);
    let end = first.end.min(second.end);
    (start < end).then_some(ByteSpan { start, end })
}

fn edges(relations: &[Pair]) -> BTreeMap<usize, BTreeSet<usize>> {
    let mut edges: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for pair in relations {
        if matches!(pair.relation, Relation::OverlapWitness { .. }) {
            edges.entry(pair.first).or_default().insert(pair.second);
            edges.entry(pair.second).or_default().insert(pair.first);
        }
    }
    edges
}

fn groups(relations: &[Pair]) -> Vec<Vec<usize>> {
    let edges = edges(relations);
    let mut remaining: BTreeSet<_> = edges.keys().copied().collect();
    let mut result = Vec::new();

    while let Some(first) = remaining.pop_first() {
        let component = component(first, &edges);
        remaining.retain(|member| !component.contains(member));
        result.push(component.into_iter().collect());
    }
    result
}

fn component(first: usize, edges: &BTreeMap<usize, BTreeSet<usize>>) -> BTreeSet<usize> {
    let mut found = BTreeSet::new();
    let mut pending = BTreeSet::from([first]);
    while let Some(next) = pending.pop_first() {
        if found.insert(next) {
            pending.extend(edges[&next].difference(&found).copied());
        }
    }
    found
}
