//! Reuse core policy over compiler-collected inputs, scoped by compilation unit.
mod diagnostic_scope;
mod diagnostics;
mod generated_effects;
mod owner_names;
mod patterns;
mod planning;
pub(super) mod propagation;
mod ranking;
mod selection;
mod source_scope;
pub(super) mod style;
#[cfg(test)]
mod tests;
use super::review::compiler_diagnostics::demand::Demand;
use super::{analysis::Completion, identity::UnitId, report::Context};
use rstyle_core::{
    advisory::{KEY, METRICS, Observation, metric_version},
    config::Config,
    facts::FnFacts,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

#[derive(Serialize)]
pub(super) struct Report {
    schema: &'static str,
    pub(super) metrics: &'static str,
    effective_config: Rc<serde_json::Value>,
    complete: bool,
    diagnostic_execution: super::analysis::Execution,
    effect_execution: super::analysis::Execution,
    pub findings: usize,
    limitations: &'static [&'static str],
    units: Vec<Unit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    planning: Option<planning::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pattern_review: Option<super::review::compiler_patterns::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic_review: Option<super::review::compiler_diagnostics::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic_baseline: Option<super::review::compiler_baseline::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pattern_baseline: Option<super::review::compiler_pattern_baseline::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    changed_scope: Option<super::analysis::changed_scope::Report<'static>>,
}

#[derive(Serialize)]
struct Unit {
    unit: UnitId,
    package_root: std::path::PathBuf,
    source_root: std::path::PathBuf,
    expected_functions: Option<usize>,
    observations: Vec<Measurement>,
    generated_effects: Vec<generated_effects::Measurement>,
    unavailable: Vec<Missing>,
    unavailable_complexity: Vec<Missing>,
    unresolved_callees: BTreeSet<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    resolved_dependency_callees: BTreeMap<String, UnitId>,
    #[serde(skip)]
    propagation_complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    evaluation: Option<rstyle_core::report::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic_scope: Option<diagnostic_scope::Report>,
    #[serde(skip)]
    diagnostic_snapshots: Vec<Result<super::review::compiler_diagnostics::Snapshot, String>>,
    #[serde(skip)]
    diagnostic_owners: Vec<super::review::compiler_diagnostics::observed::Assessed>,
    #[serde(skip)]
    diagnostic_owner_names: BTreeMap<String, usize>,
    #[serde(skip)]
    diagnostic_owner_scope_truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    visible_diagnostics: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_diagnostics: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    patterns: Option<patterns::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    complexity_ranking: Option<ranking::Report>,
}

#[derive(Serialize)]
struct Measurement {
    key: super::FunctionKey,
    name: String,
    statements: usize,
    direct_mutations: usize,
    escaping_mutations: usize,
}

#[derive(Serialize)]
struct Missing {
    key: super::FunctionKey,
    reason: String,
}

impl Completion for Report {
    fn permits_success(&self) -> bool {
        self.complete
            && self
                .pattern_baseline
                .as_ref()
                .is_none_or(|baseline| baseline.permits_success())
            && self
                .changed_scope
                .as_ref()
                .is_none_or(|scope| scope.permits_success())
            && self
                .diagnostic_baseline
                .as_ref()
                .is_none_or(|baseline| baseline.permits_success())
            && self
                .pattern_review
                .as_ref()
                .is_none_or(Completion::permits_success)
            && self
                .diagnostic_review
                .as_ref()
                .is_none_or(Completion::permits_success)
    }
}

impl Report {
    pub(super) fn baseline(&self) -> Option<&super::review::compiler_baseline::Report> {
        self.diagnostic_baseline.as_ref()
    }

    pub(super) fn pattern_baseline(
        &self,
    ) -> Option<&super::review::compiler_pattern_baseline::Report> {
        self.pattern_baseline.as_ref()
    }

    pub(super) fn with_pattern_baseline(
        mut self,
        baseline: Option<super::review::compiler_pattern_baseline::Report>,
    ) -> Self {
        self.pattern_baseline = baseline;
        self
    }

    pub(super) fn policy(&self) -> &Rc<serde_json::Value> {
        &self.effective_config
    }

    pub(super) fn render(&self) -> String {
        let summary = format!(
            "\nCompiler advisory: {} units, {} findings; complete={}. Metrics: {}.",
            self.units.len(),
            self.findings,
            self.complete,
            self.metrics
        );
        let patterns = self
            .units
            .iter()
            .filter_map(Unit::render_patterns)
            .collect::<Vec<_>>()
            .join("\n");

        let diagnostics = self
            .units
            .iter()
            .map(Unit::render_diagnostics)
            .collect::<Vec<_>>()
            .join("\n");
        let generated: String = self
            .units
            .iter()
            .map(|unit| generated_effects::render(&unit.generated_effects))
            .collect();
        let summaries = self.render_summaries();
        let planning = self
            .planning
            .as_ref()
            .map_or_else(String::new, planning::Report::render);
        let ranking: String = self
            .units
            .iter()
            .filter_map(|unit| {
                unit.complexity_ranking
                    .as_ref()
                    .map(|report| report.render(unit.unit))
            })
            .collect();
        format!("{summary}{diagnostics}{generated}{patterns}{ranking}{summaries}{planning}")
    }

    fn render_summaries(&self) -> String {
        let reviews = self
            .pattern_review
            .as_ref()
            .map_or_else(String::new, |review| review.render());
        let diagnostic_reviews = self
            .diagnostic_review
            .as_ref()
            .map_or_else(String::new, |review| review.render());
        let baseline = self
            .diagnostic_baseline
            .as_ref()
            .map_or_else(String::new, |baseline| baseline.render());

        let pattern_baseline = self
            .pattern_baseline
            .as_ref()
            .map_or_else(String::new, |baseline| baseline.render());
        let selected = self
            .changed_scope
            .as_ref()
            .map_or_else(String::new, |scope| {
                scope.render_for(
                    if self.diagnostic_execution == super::analysis::Execution::Completed {
                        "compiler diagnostics"
                    } else {
                        "compiler proposals"
                    },
                )
            });
        format!("{reviews}{diagnostic_reviews}{baseline}{pattern_baseline}{selected}")
    }

    pub(super) fn diagnostic_reviews(&self) -> Vec<super::review::compiler_diagnostics::Row<'_>> {
        self.units
            .iter()
            .flat_map(|unit| {
                unit.evaluation.iter().flat_map(move |evaluation| {
                    unit.diagnostic_snapshots
                        .iter()
                        .enumerate()
                        .filter(move |(index, _)| unit.diagnostic_requested(*index))
                        .map(
                            move |(index, snapshot)| super::review::compiler_diagnostics::Row {
                                unit: unit.unit,
                                index,
                                snapshot,
                                path: &evaluation.diagnostics[index].path,
                            },
                        )
                })
            })
            .collect()
    }

    pub(super) fn diagnostic_owner_reviews(
        &self,
        ctx: &Context<'_>,
    ) -> Vec<super::review::compiler_diagnostics::retained::Row<'_>> {
        let names = owner_names::counts(self, ctx);
        let selection = ctx
            .analysis_config
            .and_then(super::analysis::Config::changed_files);
        let direct_effects_only = ctx
            .compiler_advisory_config
            .is_some_and(|config| config.side_effects.transitive_depth == 0);
        self.units
            .iter()
            .flat_map(|unit| {
                let names = &names;
                unit.diagnostic_owners.iter().map(move |assessed| {
                    super::review::compiler_diagnostics::retained::Row {
                        assessed,
                        request: unit.diagnostic_scope.as_ref().map(|scope| &scope.request),
                        unique: owner_names::unique(names, unit, assessed),
                        snapshots: &unit.diagnostic_snapshots,
                        policy: &self.effective_config,
                        metrics: self.metrics,
                        selected: selection
                            .is_none_or(|selection| selection.contains(&assessed.path)),
                        effect_scope_complete: ctx.capture.failures.is_empty()
                            && ctx.handoff.failures.is_empty()
                            && (direct_effects_only || unit.effect_inputs_complete()),
                    }
                })
            })
            .collect()
    }

    pub(super) fn diagnostic_owner_scope_truncated(&self) -> bool {
        self.units
            .iter()
            .any(|unit| unit.diagnostic_owner_scope_truncated)
    }

    pub(super) fn pattern_reviews(
        &self,
    ) -> Vec<super::review::compiler_patterns::observed::Row<'_>> {
        self.units
            .iter()
            .flat_map(|unit| {
                unit.patterns.iter().flat_map(move |patterns| {
                    patterns
                        .evaluation
                        .candidates
                        .iter()
                        .zip(&patterns.snapshots)
                        .enumerate()
                        .map(move |(index, (candidate, snapshot))| {
                            super::review::compiler_patterns::observed::Row {
                                unit: unit.unit,
                                index,
                                id: &candidate.id,
                                candidate,
                                snapshot,
                            }
                        })
                })
            })
            .collect()
    }

    pub(in crate::clones) fn pattern_selections(&self) -> BTreeMap<UnitId, &[usize]> {
        self.units
            .iter()
            .filter_map(|unit| {
                unit.patterns
                    .as_ref()
                    .and_then(patterns::Report::selection)
                    .map(|selected| (unit.unit, selected))
            })
            .collect()
    }

    pub(super) fn advisory_inputs_complete(&self) -> bool {
        if self
            .units
            .iter()
            .all(|unit| unit.diagnostic_scope.is_none())
        {
            return self.diagnostic_execution == super::analysis::Execution::Completed
                && self.complete;
        }
        self.diagnostic_execution == super::analysis::Execution::Completed
            && !self.units.is_empty()
            && self
                .units
                .iter()
                .all(|unit| unit.expected_functions.is_some() && unit.diagnostics_complete())
    }

    pub(super) fn pattern_inputs_complete(&self) -> bool {
        !self.units.is_empty()
            && self
                .units
                .iter()
                .all(|unit| unit.expected_functions.is_some())
            && self.units.iter().any(|unit| unit.patterns.is_some())
            && self
                .units
                .iter()
                .filter_map(|unit| unit.patterns.as_ref())
                .all(|report| report.input_complete)
    }

    pub(super) fn with_reviews(
        self,
        patterns: super::review::compiler_patterns::Report,
        diagnostics: super::review::compiler_diagnostics::Report,
        baseline: Option<super::review::compiler_baseline::Report>,
    ) -> Self {
        if !patterns.requested() && !diagnostics.requested() {
            return Self {
                diagnostic_baseline: baseline,
                ..self
            };
        }
        let units: Vec<_> = self
            .units
            .into_iter()
            .map(|unit| unit.with_reviews(&patterns, &diagnostics))
            .collect();

        let findings = units
            .iter()
            .map(|unit| unit.findings(patterns::Report::findings))
            .sum();
        let complete = self.complete && patterns.permits_success() && diagnostics.permits_success();
        Self {
            units,
            findings,
            complete,
            pattern_review: patterns.requested().then_some(patterns),
            diagnostic_review: diagnostics.requested().then_some(diagnostics),
            diagnostic_baseline: baseline,
            ..self
        }
    }
}

pub(super) fn evaluate(
    ctx: &Context<'_>,
    store: Option<&super::review::ReviewStore>,
) -> Result<Option<Report>, String> {
    let Some(config) = ctx.compiler_advisory_config else {
        return Ok(None);
    };
    let policy = Rc::new(serde_json::to_value(config).map_err(|error| error.to_string())?);
    let demand = Demand::new(Rc::clone(&policy), ctx.compiler_baseline, store);
    let inputs = ctx
        .handoff
        .unit_contexts
        .iter()
        .map(|unit| collect_inputs(ctx, unit.id))
        .collect::<Result<Vec<_>, _>>()?;
    let metrics = collected_metrics(&inputs)?;
    let propagated = propagation::evaluate(ctx, &inputs, config)?;
    let (units, planning) = evaluate_units(ctx, &demand, inputs, propagated)?;
    let complete = ctx.capture.failures.is_empty()
        && ctx.handoff.failures.is_empty()
        && !units.is_empty()
        && units.iter().all(Unit::inputs_complete);

    Ok(Some(Report {
        schema: "rstyle.compiler-advisory/1",
        metrics,
        effective_config: policy,
        complete,
        diagnostic_execution: requested_execution(ctx.args.compiler_advisory),
        effect_execution: requested_execution(ctx.args.compiler_effects_requested()),
        findings: units
            .iter()
            .map(|unit| unit.findings(patterns::Report::findings))
            .sum(),
        limitations: &[
            "Advisory compiler observations and existing core policy; no strict clone or purity authority.",
            "Core effect propagation follows unique captured direct Cargo dependencies; missing or ambiguous units remain unresolved.",
            "Missing functions make requested coverage incomplete; unresolved workspace callees keep requested numeric and effect coverage incomplete, independently of complexity ranking. No zero facts are substituted.",
            "Core metrics count mutable borrows and use bounded effect propagation; they differ from strict mutation-effect metrics.",
            "Compiler patterns use approximate advisory graphs and bounded core discovery; input completeness is not exhaustive proposal coverage or refactoring safety.",
        ],
        units,
        planning,
        pattern_review: None,
        diagnostic_review: None,
        diagnostic_baseline: None,
        pattern_baseline: None,
        changed_scope: None,
    }))
}

fn requested_execution(requested: bool) -> super::analysis::Execution {
    if requested {
        super::analysis::Execution::Completed
    } else {
        super::analysis::Execution::NotRequested
    }
}

fn evaluate_units(
    ctx: &Context<'_>,
    demand: &Demand<'_>,
    inputs: Vec<Collected>,
    propagated: propagation::Evaluation,
) -> Result<(Vec<Unit>, Option<planning::Report>), String> {
    let evaluated = ctx
        .handoff
        .unit_contexts
        .iter()
        .zip(inputs)
        .zip(propagated.units)
        .map(|((unit, inputs), effects)| {
            evaluate_unit(ctx, unit, demand, Prepared { inputs, effects })
        });

    let ready = evaluated.collect::<Result<Vec<_>, _>>()?;
    let planning = propagated
        .planning
        .map(|captured| {
            let config = ctx
                .compiler_advisory_config
                .ok_or("Compiler planning policy was not requested")?;
            captured.evaluate(ctx, &ready, config)
        })
        .transpose()?;
    let units = patterns::workspace::finish(ctx, ready)?;
    Ok((units, planning))
}

pub(super) fn configuration(args: &super::Args) -> Result<Option<Config>, String> {
    if (args.compiler_inputs_requested() || args.compiler_advisory_only)
        && args.extraction_profile != rstyle_pdg::extraction_profile::ExtractionProfile::DataV3
    {
        return Err("Compiler advisory requires --extraction-profile hir-data-v3".into());
    }
    if !args.compiler_inputs_requested() {
        return Ok(None);
    }
    let config = match &args.compiler_advisory_config {
        None => Config::default(),
        Some(path) => {
            Config::parse(&std::fs::read_to_string(path).map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())?
        }
    };
    Ok(Some(config))
}

struct Prepared {
    inputs: Collected,
    effects: propagation::Completed,
}

fn evaluate_unit(
    ctx: &Context<'_>,
    unit: &super::identity::UnitContext,
    demand: &Demand<'_>,
    Prepared {
        mut inputs,
        mut effects,
    }: Prepared,
) -> Result<Ready, String> {
    let root = &ctx.capture.units[unit.observed_unit].target.manifest_dir;
    let unresolved_callees = effects.unresolved_callees(&inputs.facts)?;
    let ranking = ranking::Captured::new(ctx.args, &inputs);
    inputs.retain_propagated(&mut effects.generated);
    let completed =
        diagnostics::evaluate(ctx, unit, inputs.take_diagnostics(effects.values), demand)?;

    let diagnostic_scope = diagnostic_scope::Report::new(ctx.args, &inputs, &completed);
    let report = Unit {
        unit: unit.id,
        package_root: root.clone(),
        source_root: ctx.capture.plan.root.clone(),
        expected_functions: inputs.expected_functions(ctx),
        observations: inputs.observations,
        generated_effects: inputs.generated_effects,
        unavailable: inputs.unavailable,
        unavailable_complexity: inputs.unavailable_complexity,
        unresolved_callees,
        resolved_dependency_callees: effects.resolved,
        propagation_complete: effects.complete,
        evaluation: completed.evaluation,
        diagnostic_scope: None,
        diagnostic_snapshots: completed.snapshots,
        diagnostic_owners: completed.owners,
        diagnostic_owner_names: completed.owner_names,
        diagnostic_owner_scope_truncated: completed.owner_scope_truncated,
        visible_diagnostics: None,
        selected_diagnostics: None,
        patterns: None,
        complexity_ranking: None,
    }
    .with_diagnostic_scope(diagnostic_scope, ctx);
    Ok(Ready {
        unit: report,
        ranking,
        facts: completed.facts,
        diagnostic_indices: completed.diagnostic_indices,
        effects: completed.effects,
        missing_graphs: inputs.missing_graphs,
        generated_functions: inputs.generated_functions,
    })
}

struct Ready {
    unit: Unit,
    facts: Vec<FnFacts>,
    diagnostic_indices: Vec<usize>,
    ranking: Option<ranking::Captured>,
    effects: Option<std::collections::BTreeMap<String, Vec<rstyle_core::rules::Effect>>>,
    missing_graphs: Vec<Missing>,
    generated_functions: Vec<super::FunctionKey>,
}

impl Ready {
    fn finish(mut self) -> Result<(Unit, patterns::Inputs), String> {
        self.unit.complexity_ranking = self
            .ranking
            .map(|ranking| ranking.evaluate(&self.facts))
            .transpose()?;
        let input = patterns::Inputs {
            facts: self.facts,
            missing: self.missing_graphs,
            unavailable_functions: self.unit.unavailable.len(),
            generated_functions: self.generated_functions,
            effects: self.effects,
            effects_complete: self.unit.expected_functions.is_some()
                && self.unit.unavailable.is_empty()
                && self.unit.unresolved_callees.is_empty()
                && self.unit.propagation_complete,
        };
        Ok((self.unit, input))
    }
}

#[derive(Default)]
struct Collected {
    metrics: Option<&'static str>,
    facts: Vec<FnFacts>,
    observations: Vec<Measurement>,
    generated_effects: Vec<generated_effects::Measurement>,
    unavailable: Vec<Missing>,
    unavailable_complexity: Vec<Missing>,
    missing_graphs: Vec<Missing>,
    generated_functions: Vec<super::FunctionKey>,
}

fn collect_inputs(ctx: &Context<'_>, unit: UnitId) -> Result<Collected, String> {
    let mut inputs = Collected::default();
    for function in ctx
        .handoff
        .assessed
        .iter()
        .filter(|function| function.key.unit() == unit)
    {
        match observation(function, &ctx.capture.plan.root)? {
            Observation::Known { metrics, facts } => {
                let metrics = metric_version(&metrics).ok_or("Unknown compiler metric version")?;
                inputs.retain_metrics(metrics)?;
                inputs.push_known(function, *facts, ctx.args.compiler_patterns)?
            }
            Observation::Unavailable { reason } => {
                inputs.push_unavailable(function, reason, ctx)?;
            }
        }
    }
    Ok(inputs)
}

fn collected_metrics(inputs: &[Collected]) -> Result<&'static str, String> {
    let mut versions = inputs.iter().filter_map(|input| input.metrics);
    let metrics = versions.next().unwrap_or(METRICS);
    if versions.any(|version| version != metrics) {
        return Err("Mixed compiler metric versions require separate analyses".into());
    }
    Ok(metrics)
}

impl Collected {
    fn effect_observations_complete(&self) -> bool {
        let generated: BTreeSet<_> = self
            .generated_effects
            .iter()
            .filter(|row| row.connections().is_some())
            .map(|row| row.key)
            .collect();
        self.unavailable
            .iter()
            .all(|missing| generated.contains(&missing.key))
    }

    fn expected_functions(&self, ctx: &Context<'_>) -> Option<usize> {
        (ctx.capture.failures.is_empty() && ctx.handoff.failures.is_empty())
            .then_some(self.observations.len() + self.unavailable.len())
    }

    fn retain_propagated(
        &mut self,
        effects: &mut BTreeMap<String, Vec<rstyle_core::rules::Effect>>,
    ) {
        for row in &mut self.generated_effects {
            row.propagated_effects = row
                .connections()
                .and_then(|connections| effects.remove(&connections.id));
        }
    }

    fn retain_metrics(&mut self, metrics: &'static str) -> Result<(), String> {
        if self.metrics.is_some_and(|previous| previous != metrics) {
            return Err("Mixed compiler metric versions require separate analyses".into());
        }
        self.metrics = Some(metrics);
        Ok(())
    }

    fn push_unavailable(
        &mut self,
        function: &super::Function,
        reason: String,
        ctx: &Context<'_>,
    ) -> Result<(), String> {
        if ctx.args.compiler_effects_requested() {
            self.push_generated_effects(function, ctx)?;
        }
        if (ctx.args.compiler_pattern_scope == super::CompilerSourceScope::Authored
            || ctx.args.compiler_complexity_scope == super::CompilerSourceScope::Authored
            || ctx.args.compiler_diagnostic_scope == super::CompilerSourceScope::Authored)
            && function
                .facts
                .view()
                .source_diagnostics
                .iter()
                .any(|finding| finding.construct == "macro_generated_function")
        {
            self.generated_functions.push(function.key);
        }
        self.unavailable.push(Missing {
            key: function.key,
            reason,
        });
        Ok(())
    }

    fn push_generated_effects(
        &mut self,
        function: &super::Function,
        ctx: &Context<'_>,
    ) -> Result<(), String> {
        if let Some(config) = ctx.compiler_advisory_config
            && let Some(measurement) =
                generated_effects::read(function, config, &ctx.capture.plan.root)?
        {
            if measurement.connections().is_some() {
                self.retain_metrics(METRICS)?;
            }
            self.generated_effects.push(measurement);
        }
        Ok(())
    }

    fn take_diagnostics(
        &mut self,
        effects: Option<BTreeMap<String, Vec<rstyle_core::rules::Effect>>>,
    ) -> diagnostics::Inputs {
        diagnostics::Inputs {
            facts: std::mem::take(&mut self.facts),
            effects,
            metrics: self.metrics.unwrap_or(METRICS),
        }
    }

    fn push_known(
        &mut self,
        function: &super::Function,
        mut facts: FnFacts,
        with_graph: bool,
    ) -> Result<(), String> {
        if with_graph {
            patterns::attach(function, &mut facts, &mut self.missing_graphs)?;
        }
        if self
            .metrics
            .is_some_and(rstyle_core::advisory::has_complexity)
            && facts.complexity.is_none()
        {
            self.unavailable_complexity.push(Missing {
                key: function.key,
                reason:
                    "Compiler complexity measurement is unavailable; other metrics remain usable"
                        .into(),
            });
        }
        self.retain_known(function.key, facts);
        Ok(())
    }

    fn retain_known(&mut self, key: super::FunctionKey, facts: FnFacts) {
        self.observations.push(measurement(key, &facts));
        self.facts.push(facts);
    }
}

impl Unit {
    fn with_diagnostic_scope(
        mut self,
        scope: Option<diagnostic_scope::Report>,
        ctx: &Context<'_>,
    ) -> Self {
        let direct_effects_only = ctx
            .compiler_advisory_config
            .is_some_and(|config| config.side_effects.transitive_depth == 0);
        self.diagnostic_scope = scope.map(|mut scope| {
            scope.finish(&self, direct_effects_only);
            self.visible_diagnostics = Some(scope.diagnostic_indices.clone());
            scope
        });
        self
    }

    fn with_reviews(
        self,
        patterns: &super::review::compiler_patterns::Report,
        diagnostics: &super::review::compiler_diagnostics::Report,
    ) -> Self {
        let visible_diagnostics = if diagnostics.requested() {
            Some(
                (0..self
                    .evaluation
                    .as_ref()
                    .map_or(0, |report| report.diagnostics.len()))
                    .filter(|index| {
                        self.diagnostic_requested(*index)
                            && !diagnostics.hidden_for(self.unit, *index)
                    })
                    .collect(),
            )
        } else {
            self.visible_diagnostics.clone()
        };
        let patterns = self.patterns.map(|report| {
            if patterns.requested() {
                report.with_reviews(self.unit, patterns)
            } else {
                report
            }
        });
        Self {
            patterns,
            visible_diagnostics,
            ..self
        }
    }

    fn render_diagnostics(&self) -> String {
        self.evaluation
            .as_ref()
            .map_or_else(String::new, |evaluation| {
                let scope = self
                    .diagnostic_scope
                    .as_ref()
                    .map_or_else(String::new, diagnostic_scope::Report::render);
                format!(
                    "{scope}{}",
                    diagnostics::render(
                        self.unit,
                        evaluation,
                        self.selected_diagnostics
                            .as_deref()
                            .or(self.visible_diagnostics.as_deref()),
                    )
                )
            })
    }

    fn render_patterns(&self) -> Option<String> {
        let report = self.patterns.as_ref()?;
        Some(format!(
            "\nCompiler patterns for population anchored at unit {}, source root {} (input complete={}):\n{}",
            self.unit.ordinal(),
            self.source_root.display(),
            report.input_complete,
            report.render()
        ))
    }

    fn inputs_complete(&self) -> bool {
        self.complexity_ranking
            .as_ref()
            .is_none_or(|report| report.input_complete)
            && self.diagnostics_complete()
            && self
                .patterns
                .as_ref()
                .is_none_or(|report| report.input_complete)
    }

    fn effect_inputs_complete(&self) -> bool {
        if let Some(scope) = &self.diagnostic_scope {
            return scope.effect_inputs_complete;
        }
        self.inputs_complete() && self.propagation_complete
    }

    fn full_diagnostics_complete(&self) -> bool {
        self.evaluation.as_ref().is_none_or(|_| {
            self.unavailable.is_empty()
                && self.unavailable_complexity.is_empty()
                && self.unresolved_callees.is_empty()
        })
    }

    fn diagnostic_requested(&self, index: usize) -> bool {
        self.diagnostic_scope
            .as_ref()
            .is_none_or(|scope| scope.diagnostic_indices.binary_search(&index).is_ok())
    }

    fn diagnostics_complete(&self) -> bool {
        self.diagnostic_scope
            .as_ref()
            .map_or(self.full_diagnostics_complete(), |scope| {
                scope.input_complete
            })
    }

    fn findings(&self, count_patterns: fn(&patterns::Report) -> usize) -> usize {
        self.selected_diagnostics
            .as_ref()
            .or(self.visible_diagnostics.as_ref())
            .map_or_else(
                || {
                    self.evaluation
                        .as_ref()
                        .map_or(0, |report| report.summary.violations)
                },
                Vec::len,
            )
            + self.patterns.as_ref().map_or(0, count_patterns)
    }
}

fn observation(function: &super::Function, root: &std::path::Path) -> Result<Observation, String> {
    let Some(raw) = function.facts.view().build.frontend_options.get(KEY) else {
        return Ok(Observation::Unavailable {
            reason: "Compiler advisory observations were not captured".into(),
        });
    };
    let mut observation = Observation::read(raw)?;
    if let Observation::Known { facts, metrics } = &mut observation {
        if facts.name != function.facts.view().name
            || facts.source != function.facts.view().source.bytes.as_str()
        {
            return Err(
                "Compiler advisory metrics or source owner disagree with captured function".into(),
            );
        }
        facts.path = source_path(
            function,
            root,
            matches!(
                metrics.as_str(),
                rstyle_core::advisory::V5_METRICS | rstyle_core::advisory::V6_METRICS | METRICS
            ),
        )?;
        if facts.params + usize::from(facts.has_self)
            != function.facts.view().signature.params.len()
            || facts.param_list.len() != facts.params
            || facts.pdg.is_some()
        {
            return Err(
                "Compiler advisory signature or graph differs from declared capture".into(),
            );
        }
    }
    Ok(observation)
}

fn measurement(key: super::FunctionKey, facts: &FnFacts) -> Measurement {
    Measurement {
        key,
        name: facts.name.clone(),
        statements: facts.statements,
        direct_mutations: facts.mutations.len(),
        escaping_mutations: facts.mutations.iter().filter(|event| event.escapes).count(),
    }
}

fn unresolved(facts: &[FnFacts]) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    for function in facts {
        if function.id.is_empty() || !ids.insert(function.id.as_str()) {
            return Err(
                "Compiler advisory has missing or duplicate function identities in one unit".into(),
            );
        }
    }
    ids.extend(
        facts
            .iter()
            .filter_map(|function| function.implements.as_deref()),
    );

    Ok(facts
        .iter()
        .flat_map(|function| &function.edges)
        .filter(|edge| !ids.contains(edge.callee.as_str()))
        .map(|edge| edge.callee.clone())
        .collect())
}

fn source_path(
    function: &super::Function,
    root: &std::path::Path,
    normalize: bool,
) -> Result<String, String> {
    let source = function
        .facts
        .view()
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing compiler advisory source location")?;
    let original = std::path::Path::new(source);
    let normalized = normalize
        .then(|| rstyle_pdg::nominal_binding::absolute_path(original))
        .transpose()?;
    let path = normalized.as_deref().unwrap_or(original);

    Ok(path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/"))
}
