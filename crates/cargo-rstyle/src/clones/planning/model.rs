//! Bounded advisory relations between already checked required edit regions.
use rstyle_pdg::ByteSpan;
use serde::Serialize;

pub(super) const MAX_PLANS: usize = 4096;
pub(super) const MAX_COMPARISONS: usize = 65_536;
pub(super) const MAX_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;

/// Physical source path relative to the captured workspace, independent of units.
pub(super) struct Region<'a> {
    pub resource: &'a str,
    pub snapshot: &'a str,
    pub span: ByteSpan,
}

/// Each intent declares its complete required source footprint.
pub(super) struct Plan<'a> {
    pub id: usize,
    pub required_regions: usize,
    pub regions: Vec<Region<'a>>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Execution {
    Completed,
    BudgetExhausted,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "relation", rename_all = "snake_case")]
pub(super) enum Relation {
    OverlapWitness { witnesses: Vec<Witness> },
    DisjointKnownRegions,
    ConflictingSnapshots,
    Unavailable,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct Witness {
    pub first_region: usize,
    pub second_region: usize,
    pub intersection: ByteSpan,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct Pair {
    pub first: usize,
    pub second: usize,
    #[serde(flatten)]
    pub relation: Relation,
}

#[derive(Debug, Serialize)]
pub(super) struct Analysis {
    pub execution: Execution,
    pub coverage_complete: bool,
    pub expected_comparisons: Option<usize>,
    pub assessed_comparisons: usize,
    pub region_comparisons: usize,
    pub snapshot_comparison_bytes: usize,
    pub conflicting_snapshot_plans: Vec<usize>,
    pub unavailable_plans: Vec<usize>,
    pub relations: Vec<Pair>,
    pub overlap_groups: Vec<Vec<usize>>,
}
