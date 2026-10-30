//! Common review accounting; each consumer retains its own evidence and selectors.
use crate::clones::analysis::{Coverage, Execution};
use serde::Serialize;

pub(super) trait Decision {
    fn state(&self) -> &'static str;
}

pub(super) struct Scope {
    pub requested: bool,
    pub complete: bool,
}

pub(super) struct Contract {
    pub schema: &'static str,
    pub incomplete_scope: &'static str,
}

#[derive(Serialize)]
pub(super) struct Summary {
    schema: &'static str,
    execution: Execution,
    pub coverage: Coverage,
    findings: usize,
    pub hidden_findings: usize,
    pub visible_findings: usize,
    pub retained_entries: usize,
    reasons: Vec<&'static str>,
}

pub(super) fn summarize<D: Decision>(
    decisions: &[D],
    retained_entries: usize,
    scope: Scope,
    contract: Contract,
) -> Summary {
    let hidden_findings = decisions
        .iter()
        .filter(|decision| matches!(decision.state(), "hidden_exact" | "hidden_growth"))
        .count();
    let complete = scope.complete
        && decisions
            .iter()
            .all(|decision| decision.state() != "unavailable");
    Summary {
        schema: contract.schema,
        execution: if scope.requested {
            Execution::Completed
        } else {
            Execution::NotRequested
        },
        coverage: if scope.requested && complete {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        },
        findings: decisions.len(),
        hidden_findings,
        visible_findings: decisions.len() - hidden_findings,
        retained_entries,
        reasons: if scope.requested && !scope.complete {
            vec![contract.incomplete_scope]
        } else {
            Vec::new()
        },
    }
}

impl Summary {
    pub fn render(&self, label: &str) -> String {
        if !self.requested() {
            return String::new();
        }
        format!(
            "\n{label} reviews: {:?} coverage; {} visible, {} hidden; {} retained entries.",
            self.coverage, self.visible_findings, self.hidden_findings, self.retained_entries
        )
    }

    pub fn requested(&self) -> bool {
        self.execution != Execution::NotRequested
    }

    pub fn visible_or(&self, visible: usize, unfiltered: usize) -> usize {
        if self.requested() {
            visible
        } else {
            unfiltered
        }
    }
}

impl crate::clones::analysis::Completion for Summary {
    fn permits_success(&self) -> bool {
        !self.requested() || self.coverage == Coverage::Complete
    }
}
