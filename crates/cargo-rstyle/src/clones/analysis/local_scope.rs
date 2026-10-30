//! Common lifecycle for opt-in consumers of captured original function owners.
use super::{Coverage, Execution};
use crate::clones::{Handoff, capture};
use serde::Serialize;
use std::marker::PhantomData;

pub(super) trait Policy {
    const POPULATION: &'static str;
    const UNAVAILABLE_REASON: &'static str;
}

pub(super) struct Status {
    pub assessed: bool,
    pub evaluated: bool,
}

pub(super) trait FunctionStatus {
    fn status(&self) -> Status;
}

#[derive(Serialize)]
pub(super) struct Summary<P: Policy> {
    pub execution: Execution,
    pub coverage: Coverage,
    population: &'static str,
    expected_functions: Option<usize>,
    assessed_functions: usize,
    pub evaluated_functions: usize,
    pub unavailable_functions: usize,
    reasons: Vec<String>,
    #[serde(skip)]
    policy: PhantomData<P>,
}

#[derive(Default)]
struct Counts {
    population: usize,
    assessed: usize,
    evaluated: usize,
}

impl Counts {
    fn include(self, status: Status) -> Self {
        Self {
            population: self.population + 1,
            assessed: self.assessed + usize::from(status.assessed),
            evaluated: self.evaluated + usize::from(status.evaluated),
        }
    }
}

impl<P: Policy> Summary<P> {
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
            population: P::POPULATION,
            expected_functions: reasons.is_empty().then_some(handoff.assessed.len()),
            assessed_functions: 0,
            evaluated_functions: 0,
            unavailable_functions: 0,
            reasons,
            policy: PhantomData,
        }
    }

    pub fn finish(&mut self, functions: &[impl FunctionStatus], extra_unavailable: bool) {
        if self.execution == Execution::NotRequested {
            return;
        }
        let counts = functions
            .iter()
            .fold(Counts::default(), |counts, function| {
                counts.include(function.status())
            });
        let unavailable_functions = counts.population - counts.evaluated;
        let execution =
            if self.execution != Execution::Failed && counts.assessed < counts.population {
                Execution::BudgetExhausted
            } else {
                self.execution
            };
        let mut reasons = std::mem::take(&mut self.reasons);
        if unavailable_functions > 0 || extra_unavailable {
            reasons.push(P::UNAVAILABLE_REASON.into());
        }

        *self = Self {
            execution,
            coverage: if reasons.is_empty() && execution == Execution::Completed {
                Coverage::Complete
            } else {
                Coverage::Incomplete
            },
            assessed_functions: counts.assessed,
            evaluated_functions: counts.evaluated,
            unavailable_functions,
            reasons,
            ..*self
        };
    }

    /// Pair-level incompleteness augments the same pending whole-function scope.
    pub fn record_boundary(&mut self, execution: Execution, reason: Option<&str>) {
        let execution =
            if execution == Execution::BudgetExhausted && self.execution != Execution::Failed {
                Execution::BudgetExhausted
            } else {
                self.execution
            };
        let mut reasons = std::mem::take(&mut self.reasons);
        if let Some(reason) = reason {
            reasons.push(reason.into());
        }

        *self = Self {
            execution,
            reasons,
            ..*self
        };
    }
}

impl<P: Policy> super::Completion for Summary<P> {
    fn permits_success(&self) -> bool {
        self.execution == Execution::NotRequested
            || (self.execution == Execution::Completed && self.coverage == Coverage::Complete)
    }
}
