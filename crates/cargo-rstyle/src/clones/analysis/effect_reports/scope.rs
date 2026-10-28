use super::{Coverage, Execution, FunctionReport};
use crate::clones::{Handoff, capture};
use rstyle_pdg::consumer_facts::{Availability, mutations};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Summary {
    pub execution: Execution,
    pub coverage: Coverage,
    population: &'static str,
    expected_functions: Option<usize>,
    assessed_functions: usize,
    pub evaluated_functions: usize,
    pub unavailable_functions: usize,
    reasons: Vec<String>,
}

impl Summary {
    pub fn new(handoff: &Handoff, capture: &capture::Capture, requested: bool) -> Self {
        let reasons: Vec<_> = capture
            .failures
            .iter()
            .chain(&handoff.failures)
            .cloned()
            .collect();
        Self {
            execution: if !requested {
                Execution::NotRequested
            } else if reasons.is_empty() {
                Execution::Completed
            } else {
                Execution::Failed
            },
            coverage: Coverage::Incomplete,
            population: "captured original function owners, independent of strict graph admission",
            expected_functions: reasons.is_empty().then_some(handoff.assessed.len()),
            assessed_functions: 0,
            evaluated_functions: 0,
            unavailable_functions: 0,
            reasons,
        }
    }

    pub fn finish(&mut self, functions: &[FunctionReport<'_>]) {
        if self.execution == Execution::NotRequested {
            return;
        }
        let assessed_functions = functions
            .iter()
            .filter(|function| function.evidence.execution != mutations::Execution::BudgetExhausted)
            .count();
        let evaluated_functions = functions
            .iter()
            .filter(|function| complete(function))
            .count();
        let unavailable_functions = functions.len() - evaluated_functions;
        let execution =
            if self.execution != Execution::Failed && assessed_functions < functions.len() {
                Execution::BudgetExhausted
            } else {
                self.execution
            };
        let mut reasons = std::mem::take(&mut self.reasons);
        if unavailable_functions > 0 {
            reasons.push("Some owners have incomplete scalar mutation effects; inspect their boundaries and reasons".into());
        }

        *self = Self {
            execution,
            coverage: if reasons.is_empty() && execution == Execution::Completed {
                Coverage::Complete
            } else {
                Coverage::Incomplete
            },
            assessed_functions,
            evaluated_functions,
            unavailable_functions,
            reasons,
            ..*self
        };
    }
}

fn complete(function: &FunctionReport<'_>) -> bool {
    let evidence = &function.evidence;
    evidence.coverage == mutations::Coverage::Complete
        && matches!(evidence.availability, Availability::Known(()))
        && evidence.execution == mutations::Execution::Completed
}
