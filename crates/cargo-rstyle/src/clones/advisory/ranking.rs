//! Project existing core complexity ordering without copying compiler bodies.
use super::{Collected, source_scope::SourceScope};
use crate::clones::FunctionKey;
use rstyle_core::{complexity_report, facts::FnFacts};
use serde::Serialize;
use std::{collections::BTreeMap, num::NonZeroUsize};

pub(super) struct Captured {
    limit: NonZeroUsize,
    owners: BTreeMap<String, FunctionKey>,
    unavailable: usize,
    source_scope: Option<SourceScope>,
}

#[derive(Serialize)]
pub(super) struct Report {
    schema: &'static str,
    requested_limit: usize,
    known_functions: usize,
    measured_functions: usize,
    unavailable_measurements: usize,
    unavailable_owners: usize,
    pub input_complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_scope: Option<SourceScope>,
    owners: Vec<FunctionKey>,
    functions: Vec<complexity_report::Function>,
}

impl Captured {
    pub(super) fn new(args: &crate::clones::Args, inputs: &Collected) -> Option<Self> {
        let limit = args.compiler_complexity_top?;
        let source_scope = SourceScope::select(
            args.compiler_complexity_scope,
            inputs.generated_functions.iter().copied(),
        );
        let outside = source_scope
            .as_ref()
            .map_or(0, |scope| scope.outside_scope_functions.len());
        Some(Self {
            limit,
            owners: inputs
                .facts
                .iter()
                .zip(&inputs.observations)
                .map(|(fact, observation)| (fact.id.clone(), observation.key))
                .collect(),
            unavailable: inputs.unavailable.len() - outside,
            source_scope,
        })
    }

    pub(super) fn evaluate(self, facts: &[FnFacts]) -> Result<Report, String> {
        let mut report = Report::measurements(facts, self.limit, self.unavailable);
        report.source_scope = self.source_scope;
        for fact in complexity_report::ranked(facts)
            .into_iter()
            .take(self.limit.get())
        {
            let key = self
                .owners
                .get(&fact.id)
                .ok_or("Ranked compiler function has no original report owner")?;
            report.owners.push(*key);
            report
                .functions
                .push(complexity_report::Function::from(fact));
        }
        Ok(report)
    }
}

impl Report {
    fn measurements(facts: &[FnFacts], limit: NonZeroUsize, unavailable: usize) -> Self {
        let measured_functions = facts
            .iter()
            .filter(|fact| fact.complexity.is_some())
            .count();
        Self {
            schema: "rstyle.compiler-complexity-ranking/1",
            requested_limit: limit.get(),
            known_functions: facts.len(),
            measured_functions,
            unavailable_measurements: facts.len() - measured_functions,
            unavailable_owners: unavailable,
            input_complete: measured_functions == facts.len() && unavailable == 0,
            source_scope: None,
            owners: Vec::new(),
            functions: Vec::new(),
        }
    }

    pub(super) fn render(&self, unit: super::UnitId) -> String {
        let scope = self
            .source_scope
            .as_ref()
            .map_or_else(String::new, |scope| {
                scope.render(self.unavailable_owners + self.unavailable_measurements)
            });
        format!(
            "\nCompiler complexity ranking for unit {} (top {}, measured {}/{}, unavailable owners {}, input complete={}):\n{scope}{}\n",
            unit.ordinal(),
            self.requested_limit,
            self.measured_functions,
            self.known_functions,
            self.unavailable_owners,
            self.input_complete,
            complexity_report::text(&self.functions).join("\n")
        )
    }
}

#[cfg(test)]
mod tests;
