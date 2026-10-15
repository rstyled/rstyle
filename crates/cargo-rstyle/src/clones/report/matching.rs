//! Borrowed readable matching exports; no intermediate JSON tree.
use rstyle_ccgraph::{Decision, PairRecord, RunReport, StageOutcome, StageRecord};
use serde::{
    Serialize, Serializer,
    ser::{SerializeMap, SerializeSeq},
};
use std::collections::BTreeMap;

pub(super) struct Matching<'a> {
    pub report: &'a RunReport,
    pub trace: bool,
    pub partitions: BTreeMap<usize, Vec<rstyle_ccgraph::wl::LabelPartition>>,
}

impl<'a> Matching<'a> {
    pub fn new(
        report: &'a RunReport,
        trace: bool,
        partitions: rstyle_ccgraph::pipeline::WlPartitions,
    ) -> Self {
        Self {
            report,
            trace,
            partitions,
        }
    }
}

impl Serialize for Matching<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let RunReport {
            profile_id,
            profile_version,
            scoring,
            effective_config,
            functions,
            pairs: _,
            incomplete,
            diagnostics,
        } = self.report;
        #[derive(Serialize)]
        struct Export<'a> {
            profile_id: &'a str,
            profile_version: &'a str,
            scoring: &'a str,
            effective_config: &'a BTreeMap<String, String>,
            functions: &'a [rstyle_ccgraph::FunctionRecord],
            pairs: Pairs<'a>,
            incomplete: bool,
            diagnostics: &'a [String],
        }

        Export {
            profile_id,
            profile_version,
            scoring,
            effective_config,
            functions,
            pairs: Pairs(self),
            incomplete: *incomplete,
            diagnostics,
        }
        .serialize(serializer)
    }
}

struct Pairs<'a>(&'a Matching<'a>);
impl Serialize for Pairs<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.report.pairs.len()))?;
        for (index, pair) in self.0.report.pairs.iter().enumerate() {
            seq.serialize_element(&Pair {
                pair,
                trace: self.0.trace,
                partitions: self.0.partitions.get(&index).map(Vec::as_slice),
            })?;
        }
        seq.end()
    }
}

struct Pair<'a> {
    pair: &'a PairRecord,
    trace: bool,
    partitions: Option<&'a [rstyle_ccgraph::wl::LabelPartition]>,
}
impl Serialize for Pair<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let PairRecord {
            first,
            second,
            stages,
            decision,
            displayed_score,
        } = self.pair;
        let status = evaluation_status(stages);
        let decision: Option<&Decision> = (status == "evaluated").then_some(decision);
        #[derive(Serialize)]
        struct Export<'a> {
            first: &'a str,
            second: &'a str,
            stages: Stages<'a>,
            decision: Option<&'a Decision>,
            displayed_score: &'a Option<String>,
            evaluation_status: &'static str,
            #[serde(skip_serializing_if = "Option::is_none")]
            wl_label_partitions: Option<&'a [rstyle_ccgraph::wl::LabelPartition]>,
        }

        Export {
            first,
            second,
            stages: Stages {
                stages,
                trace: self.trace,
            },
            decision,
            displayed_score,
            evaluation_status: status,
            wl_label_partitions: self.partitions,
        }
        .serialize(serializer)
    }
}

fn evaluation_status(stages: &[StageRecord]) -> &'static str {
    if stages
        .iter()
        .any(|s| matches!(s.outcome, StageOutcome::Error { .. }))
    {
        return "failed";
    }
    if stages
        .iter()
        .any(|s| matches!(s.outcome, StageOutcome::Ineligible { .. }))
    {
        return "incompatible";
    }
    "evaluated"
}

struct Stages<'a> {
    stages: &'a [StageRecord],
    trace: bool,
}
impl Serialize for Stages<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.stages.len()))?;
        for stage in self.stages {
            seq.serialize_element(&Stage {
                record: stage,
                trace: self.trace,
            })?;
        }
        seq.end()
    }
}

struct Stage<'a> {
    record: &'a StageRecord,
    trace: bool,
}
impl Serialize for Stage<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let StageRecord {
            stage,
            outcome,
            detail,
        } = self.record;

        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("stage", stage)?;
        map.serialize_entry("outcome", outcome)?;
        map.serialize_entry(
            "detail",
            &Detail {
                entries: detail,
                trace: self.trace,
            },
        )?;
        map.end()
    }
}

struct Detail<'a> {
    entries: &'a BTreeMap<String, String>,
    trace: bool,
}
impl Serialize for Detail<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        for (key, value) in self.entries {
            if self.trace || !key.starts_with("round_") {
                map.serialize_entry(key, value)?;
            }
        }
        map.end()
    }
}

#[cfg(test)]
mod tests;
