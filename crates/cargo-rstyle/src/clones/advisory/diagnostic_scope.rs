//! Project requested rules from the completed core result without changing its summary.
use super::{Collected, Unit, diagnostics, source_scope::SourceScope};
use crate::clones::{Args, diagnostic_selection::Request};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Report {
    pub request: Request,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_scope: Option<SourceScope>,
    pub diagnostic_indices: Vec<usize>,
    pub input_complete: bool,
    pub(super) effect_inputs_complete: bool,
    unavailable_owners: usize,
    unavailable_measurements: usize,
}

impl Report {
    pub(super) fn new(
        args: &Args,
        inputs: &Collected,
        completed: &diagnostics::Completed,
    ) -> Option<Self> {
        let request = args.compiler_diagnostic_request()?;
        let source_scope = SourceScope::select(
            request.source_scope,
            inputs.generated_functions.iter().copied(),
        );
        let unavailable_owners = inputs.unavailable.len()
            - source_scope
                .as_ref()
                .map_or(0, |scope| scope.outside_scope_functions.len());
        let unavailable_measurements = if request.needs_complexity() {
            completed
                .facts
                .iter()
                .filter(|fact| fact.complexity.is_none())
                .count()
        } else {
            0
        };

        let diagnostic_indices = completed
            .evaluation
            .iter()
            .flat_map(|evaluation| evaluation.diagnostics.iter().enumerate())
            .filter(|(_, diagnostic)| request.includes(&diagnostic.rule_id))
            .map(|(index, _)| index)
            .collect();
        Some(Self {
            request,
            source_scope,
            diagnostic_indices,
            input_complete: unavailable_owners == 0 && unavailable_measurements == 0,
            effect_inputs_complete: false,
            unavailable_owners,
            unavailable_measurements,
        })
    }

    pub(super) fn finish(&mut self, unit: &Unit) {
        self.effect_inputs_complete = unit.expected_functions.is_some()
            && unit.unavailable.is_empty()
            && unit.unresolved_callees.is_empty()
            && unit.propagation_complete;
        self.input_complete &= unit.expected_functions.is_some()
            && (!self.request.includes("side_effect_density") || self.effect_inputs_complete);
    }

    pub(super) fn outside_scope(&self) -> impl Iterator<Item = crate::clones::FunctionKey> + '_ {
        self.source_scope
            .iter()
            .flat_map(|scope| scope.outside_scope_functions.iter().copied())
    }

    pub(super) fn render(&self) -> String {
        let source = self
            .source_scope
            .as_ref()
            .map_or_else(String::new, |scope| {
                scope.render(self.unavailable_owners + self.unavailable_measurements)
            });
        format!(
            "\nRequested compiler diagnostics: {:?}; input complete={}; core evaluation and summary retain all rules.\n{source}",
            self.request.rules, self.input_complete
        )
    }
}
