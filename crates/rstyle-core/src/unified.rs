//! One pass over a run's records: each selected analysis hands what it completed to the
//! next, so core derives facts and effects once however many analyses read them.

use crate::config::Config;
use crate::cpg::{self, RunInputs, records::Record};
use crate::filter::{Analysis, Filter};
use crate::patterns_report::{self, PatternsReport};
use crate::report::Report;
use crate::{Retention, candidates, rules};
use std::collections::BTreeMap;

/// What the selected analyses reported; a filtered one stays absent.
#[derive(Debug, Default)]
pub struct Analyses {
    pub check: Option<Report>,
    pub patterns: Option<PatternsReport>,
}

/// Runs the selected check and pattern analyses. Runtime bounds read the records themselves,
/// so a caller wanting them takes them first.
pub fn analyze(
    records: Vec<Record>,
    filter: &Filter,
    config: &Config,
    options: &patterns_report::Options,
) -> Result<Analyses, String> {
    let check = filter.core(Analysis::Check);
    if !filter.core(Analysis::Patterns) {
        return match check {
            true => Ok(Analyses {
                check: Some(checked(cpg::run_inputs(records, config)?, config)?),
                patterns: None,
            }),
            false => Ok(Analyses::default()),
        };
    }

    let inputs = cpg::run_inputs(records, config)?;
    let completed = match check {
        true => check_then_observed(inputs, config)?,
        false => observed(inputs, config)?,
    };
    let (mut patterns, _) =
        patterns_report::run_distinct_owned_and_retain(completed.facts, options);
    candidates::annotate_completed_effects(&mut patterns.candidates, &completed.effects)?;
    Ok(Analyses {
        check: completed.report,
        patterns: Some(patterns),
    })
}

fn checked(inputs: RunInputs, config: &Config) -> Result<Report, String> {
    crate::evaluate_with_effects(inputs.facts, config, Retention::Report, inputs.effects)
        .map(|evaluation| evaluation.report)
}

struct Completed {
    report: Option<Report>,
    facts: Vec<crate::facts::FnFacts>,
    effects: BTreeMap<String, Vec<rules::Effect>>,
}

fn check_then_observed(inputs: RunInputs, config: &Config) -> Result<Completed, String> {
    let evaluation = crate::evaluate_retaining_effects(
        inputs.facts,
        config,
        Retention::Report,
        Some(inputs.effects),
    )?;
    Ok(Completed {
        report: Some(evaluation.report),
        facts: evaluation.facts,
        effects: evaluation
            .effects
            .ok_or("The evaluation did not retain its effects")?,
    })
}

fn observed(inputs: RunInputs, config: &Config) -> Result<Completed, String> {
    let observed = crate::effect_observations(inputs.facts, config, Some(inputs.effects))?;
    Ok(Completed {
        report: None,
        facts: observed.facts,
        effects: observed.effects,
    })
}
