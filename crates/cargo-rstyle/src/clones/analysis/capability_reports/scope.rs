use super::{FunctionReport, capability_pairs};
use crate::clones::{Handoff, analysis::local_scope, capture};
use serde::Serialize;
use std::ops::Deref;

pub(super) struct Policy;

impl local_scope::Policy for Policy {
    const POPULATION: &'static str =
        "original captured function owners; ordered wrapper pairs within each selected unit";
    const UNAVAILABLE_REASON: &'static str =
        "Some owners or pairs lack complete capability evidence; inspect their reasons";
}

#[derive(Serialize)]
pub(super) struct Summary {
    #[serde(flatten)]
    functions: local_scope::Summary<Policy>,
    expected_pairs: Option<usize>,
    assessed_pairs: usize,
}

impl Deref for Summary {
    type Target = local_scope::Summary<Policy>;

    fn deref(&self) -> &Self::Target {
        &self.functions
    }
}

impl Summary {
    pub fn new(handoff: &Handoff, capture: &capture::Capture, requested: bool) -> Self {
        Self {
            functions: local_scope::Summary::new(handoff, capture, requested),
            expected_pairs: Some(0),
            assessed_pairs: 0,
        }
    }

    pub fn accept_pairs(&mut self, pairs: &capability_pairs::Report) {
        let expected_pairs = self
            .expected_pairs
            .zip(pairs.expected_pairs)
            .and_then(|(left, right)| left.checked_add(right));
        let assessed_pairs = self.assessed_pairs + pairs.assessed_pairs;
        let reason =
            (!pairs.complete).then_some("Some ordered capability pairs remain unevaluated");

        self.functions.record_boundary(pairs.execution, reason);
        self.expected_pairs = expected_pairs;
        self.assessed_pairs = assessed_pairs;
    }

    pub fn finish(&mut self, functions: &[FunctionReport<'_>], unavailable_pairs: usize) {
        self.functions.finish(functions, unavailable_pairs > 0);
    }
}
