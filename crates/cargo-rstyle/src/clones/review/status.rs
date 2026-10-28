//! Read-only accounting for every retained review, independent of finding exits.
use super::Locator;
use crate::clones::analysis::{Coverage, Execution};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::clones) enum Family {
    Helper,
    EnumDispatch,
    CapabilitySet,
    Parameter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::clones) enum State {
    HiddenExact,
    HiddenGrowth,
    Stale,
    ContextChanged,
    NoFinding,
    NotObserved,
    NotRequested,
    Unavailable,
}

impl State {
    fn assessed(self) -> bool {
        matches!(
            self,
            Self::HiddenExact
                | Self::HiddenGrowth
                | Self::Stale
                | Self::ContextChanged
                | Self::NoFinding
        )
    }
}

#[derive(Debug, Serialize)]
pub(in crate::clones) struct Record {
    pub family: Family,
    /// Zero-based position in the corresponding retained storage vector.
    pub entry_index: usize,
    pub state: State,
    pub detail: String,
    /// Original dismissal reason; status assessment never replaces this choice.
    pub reason: String,
    pub members: Vec<Locator>,
}

#[derive(Debug, Serialize)]
pub(in crate::clones) struct Report {
    pub schema: &'static str,
    pub execution: Execution,
    pub coverage: Coverage,
    pub retained_entries: usize,
    pub assessed_entries: usize,
    pub unassessed_entries: usize,
    pub records: Vec<Record>,
}

impl Report {
    pub fn new(records: Vec<Record>, requested: bool) -> Self {
        let assessed_entries = records
            .iter()
            .filter(|record| record.state.assessed())
            .count();
        let unassessed_entries = records.len() - assessed_entries;
        Self {
            schema: "rstyle.review-status/1",
            execution: if requested {
                Execution::Completed
            } else {
                Execution::NotRequested
            },
            coverage: if requested && unassessed_entries == 0 {
                Coverage::Complete
            } else {
                Coverage::Incomplete
            },
            retained_entries: records.len(),
            assessed_entries,
            unassessed_entries,
            records,
        }
    }

    pub fn render(&self) -> String {
        if self.execution == Execution::NotRequested {
            return String::new();
        }
        let mut output = format!(
            "\nRetained review status: {:?} coverage; {} assessed, {} unassessed; {} retained entries.",
            self.coverage, self.assessed_entries, self.unassessed_entries, self.retained_entries
        );
        for record in &self.records {
            output.push_str(&format!(
                "\nRetained {:?} review {}: {:?}. {} Dismissal: {}",
                record.family, record.entry_index, record.state, record.detail, record.reason
            ));
        }
        output
    }
}

#[cfg(test)]
mod tests;

impl crate::clones::analysis::Completion for Report {
    fn permits_success(&self) -> bool {
        self.execution == Execution::NotRequested || self.coverage == Coverage::Complete
    }
}
