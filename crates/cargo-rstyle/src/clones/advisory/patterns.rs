//! Adapt owned compiler graphs to existing core proposal policy.
use super::Missing;
use rstyle_core::{
    advisory::patterns::{KEY, Observation},
    facts::FnFacts,
    patterns_report,
};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Report {
    schema: &'static str,
    policy: rstyle_core::cluster::Params,
    pub input_complete: bool,
    policy_selected_functions: usize,
    observed_graphs: usize,
    unavailable_functions: usize,
    unavailable_graphs: Vec<Missing>,
    pub evaluation: patterns_report::PatternsReport,
}

pub(super) fn attach(
    function: &crate::clones::Function,
    facts: &mut FnFacts,
    missing: &mut Vec<Missing>,
) -> Result<(), String> {
    let Some(raw) = function.facts.build.frontend_options.get(KEY) else {
        missing.push(Missing {
            key: function.key,
            reason: "Compiler pattern graph was not captured".into(),
        });
        return Ok(());
    };
    let observation = Observation::read(raw)?;
    if observation.owner != facts.id {
        return Err("Compiler pattern graph belongs to another function".into());
    }

    facts.pdg = Some(observation.graph);
    Ok(())
}

pub(super) fn evaluate(
    facts: Vec<FnFacts>,
    missing: Vec<Missing>,
    unavailable_functions: usize,
) -> Report {
    let options = patterns_report::Options::default();
    Report {
        schema: "rstyle.compiler-patterns/1",
        policy: options.params,
        input_complete: missing.is_empty() && unavailable_functions == 0,
        policy_selected_functions: facts.len(),
        observed_graphs: facts
            .iter()
            .filter(|function| function.pdg.is_some())
            .count(),
        unavailable_functions,
        unavailable_graphs: missing,
        evaluation: patterns_report::run_owned(facts, &options),
    }
}
