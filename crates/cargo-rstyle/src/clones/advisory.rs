//! Reuse core policy over compiler-collected inputs, scoped by compilation unit.
mod diagnostics;
mod owner_names;
mod patterns;
mod planning;
mod propagation;
mod selection;
#[cfg(test)]
mod tests;
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
    unavailable: Vec<Missing>,
    unavailable_complexity: Vec<Missing>,
    unresolved_callees: BTreeSet<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    resolved_dependency_callees: BTreeMap<String, UnitId>,
    #[serde(skip)]
    propagation_complete: bool,
    evaluation: rstyle_core::report::Report,
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
        let summaries = self.render_summaries();
        let planning = self
            .planning
            .as_ref()
            .map_or_else(String::new, planning::Report::render);
        format!("{summary}{diagnostics}{patterns}{summaries}{planning}")
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
                scope.render_for("compiler diagnostics")
            });
        format!("{reviews}{diagnostic_reviews}{baseline}{pattern_baseline}{selected}")
    }

    pub(super) fn diagnostic_reviews(&self) -> Vec<super::review::compiler_diagnostics::Row<'_>> {
        self.units
            .iter()
            .flat_map(|unit| {
                unit.diagnostic_snapshots
                    .iter()
                    .enumerate()
                    .map(
                        move |(index, snapshot)| super::review::compiler_diagnostics::Row {
                            unit: unit.unit,
                            index,
                            snapshot,
                            path: &unit.evaluation.diagnostics[index].path,
                        },
                    )
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
                        unique: owner_names::unique(names, unit, assessed),
                        snapshots: &unit.diagnostic_snapshots,
                        policy: &self.effective_config,
                        metrics: self.metrics,
                        selected: selection
                            .is_none_or(|selection| selection.contains(&assessed.path)),
                        effect_scope_complete: ctx.capture.failures.is_empty()
                            && ctx.handoff.failures.is_empty()
                            && (direct_effects_only
                                || (unit.inputs_complete() && unit.propagation_complete)),
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
                                snapshot,
                            }
                        })
                })
            })
            .collect()
    }

    pub(super) fn pattern_inputs_complete(&self) -> bool {
        self.complete
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

        let findings = units.iter().map(Unit::findings).sum();
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

pub(super) fn evaluate(ctx: &Context<'_>) -> Result<Option<Report>, String> {
    let Some(config) = ctx.compiler_advisory_config else {
        return Ok(None);
    };
    let policy = Rc::new(serde_json::to_value(config).map_err(|error| error.to_string())?);
    let inputs = ctx
        .handoff
        .unit_contexts
        .iter()
        .map(|unit| collect_inputs(ctx, unit.id))
        .collect::<Result<Vec<_>, _>>()?;
    let metrics = collected_metrics(&inputs)?;
    let propagated = propagation::evaluate(ctx, &inputs, config)?;
    let (units, planning) = evaluate_units(ctx, &policy, inputs, propagated)?;
    let complete = ctx.capture.failures.is_empty()
        && ctx.handoff.failures.is_empty()
        && !units.is_empty()
        && units.iter().all(Unit::inputs_complete);

    Ok(Some(Report {
        schema: "rstyle.compiler-advisory/1",
        metrics,
        effective_config: policy,
        complete,
        findings: units.iter().map(Unit::findings).sum(),
        limitations: &[
            "Advisory compiler observations and existing core policy; no strict clone or purity authority.",
            "Core effect propagation follows unique captured direct Cargo dependencies; missing or ambiguous units remain unresolved.",
            "Missing functions and unresolved workspace callees make coverage incomplete; no zero facts are substituted.",
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

fn evaluate_units(
    ctx: &Context<'_>,
    policy: &Rc<serde_json::Value>,
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
            evaluate_unit(ctx, unit, policy, Prepared { inputs, effects })
        });

    let Some(captured) = propagated.planning else {
        let units = evaluated
            .zip(&ctx.handoff.unit_contexts)
            .map(|(ready, unit)| ready.and_then(|ready| ready.finish(ctx, unit)))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok((units, None));
    };
    let ready = evaluated.collect::<Result<Vec<_>, _>>()?;
    let config = ctx
        .compiler_advisory_config
        .ok_or("Compiler planning policy was not requested")?;
    let planning = captured.evaluate(ctx, &ready, config)?;
    let units = ready
        .into_iter()
        .zip(&ctx.handoff.unit_contexts)
        .map(|(ready, unit)| ready.finish(ctx, unit))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((units, Some(planning)))
}

pub(super) fn configuration(args: &super::Args) -> Result<Option<Config>, String> {
    if !args.compiler_advisory {
        return Ok(None);
    }
    if args.extraction_profile != rstyle_pdg::extraction_profile::ExtractionProfile::DataV3 {
        return Err("Compiler advisory requires --extraction-profile hir-data-v3".into());
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
    policy: &Rc<serde_json::Value>,
    prepared: Prepared,
) -> Result<Ready, String> {
    let Prepared { inputs, effects } = prepared;
    let root = &ctx.capture.units[unit.observed_unit].target.manifest_dir;
    let unresolved_callees =
        propagation::unresolved(&inputs.facts, effects.unresolved, &effects.resolved)?;
    let expected_functions = (ctx.capture.failures.is_empty() && ctx.handoff.failures.is_empty())
        .then_some(inputs.observations.len() + inputs.unavailable.len());
    let completed = diagnostics::evaluate(
        ctx,
        unit,
        diagnostics::Inputs {
            facts: inputs.facts,
            effects: effects.values,
            metrics: inputs.metrics.unwrap_or(METRICS),
        },
        policy,
    )?;

    let report = Unit {
        unit: unit.id,
        package_root: root.clone(),
        source_root: ctx.capture.plan.root.clone(),
        expected_functions,
        observations: inputs.observations,
        unavailable: inputs.unavailable,
        unavailable_complexity: inputs.unavailable_complexity,
        unresolved_callees,
        resolved_dependency_callees: effects.resolved,
        propagation_complete: effects.complete,
        evaluation: completed.evaluation,
        diagnostic_snapshots: completed.snapshots,
        diagnostic_owners: completed.owners,
        diagnostic_owner_names: completed.owner_names,
        diagnostic_owner_scope_truncated: completed.owner_scope_truncated,
        visible_diagnostics: None,
        selected_diagnostics: None,
        patterns: None,
    };
    Ok(Ready {
        unit: report,
        facts: completed.facts,
        diagnostic_indices: completed.diagnostic_indices,
        effects: completed.effects,
        missing_graphs: inputs.missing_graphs,
    })
}

struct Ready {
    unit: Unit,
    facts: Vec<FnFacts>,
    diagnostic_indices: Vec<usize>,
    effects: Option<std::collections::BTreeMap<String, Vec<rstyle_core::rules::Effect>>>,
    missing_graphs: Vec<Missing>,
}

impl Ready {
    fn finish(
        mut self,
        ctx: &Context<'_>,
        unit: &super::identity::UnitContext,
    ) -> Result<Unit, String> {
        self.unit.patterns = evaluate_patterns(
            ctx,
            unit,
            patterns::Inputs {
                facts: self.facts,
                missing: self.missing_graphs,
                unavailable_functions: self.unit.unavailable.len(),
                effects: self.effects,
                effects_complete: self.unit.expected_functions.is_some()
                    && self.unit.unavailable.is_empty()
                    && self.unit.unresolved_callees.is_empty()
                    && self.unit.propagation_complete,
            },
        )?;
        Ok(self.unit)
    }
}

fn evaluate_patterns(
    ctx: &Context<'_>,
    unit: &super::identity::UnitContext,
    inputs: patterns::Inputs,
) -> Result<Option<patterns::Report>, String> {
    ctx.args
        .compiler_patterns
        .then(|| patterns::evaluate(ctx, unit, inputs))
        .transpose()
}

#[derive(Default)]
struct Collected {
    metrics: Option<&'static str>,
    facts: Vec<FnFacts>,
    observations: Vec<Measurement>,
    unavailable: Vec<Missing>,
    unavailable_complexity: Vec<Missing>,
    missing_graphs: Vec<Missing>,
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
                if inputs.metrics.is_some_and(|previous| previous != metrics) {
                    return Err("Mixed compiler metric versions require separate analyses".into());
                }
                inputs.metrics = Some(metrics);
                inputs.push_known(function, *facts, ctx.args.compiler_patterns)?
            }
            Observation::Unavailable { reason } => inputs.unavailable.push(Missing {
                key: function.key,
                reason,
            }),
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
    fn push_known(
        &mut self,
        function: &super::Function,
        mut facts: FnFacts,
        with_graph: bool,
    ) -> Result<(), String> {
        if with_graph {
            patterns::attach(function, &mut facts, &mut self.missing_graphs)?;
        }
        if self.metrics == Some(METRICS) && facts.complexity.is_none() {
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
    fn with_reviews(
        self,
        patterns: &super::review::compiler_patterns::Report,
        diagnostics: &super::review::compiler_diagnostics::Report,
    ) -> Self {
        let visible_diagnostics = diagnostics.requested().then(|| {
            (0..self.evaluation.diagnostics.len())
                .filter(|index| !diagnostics.hidden_for(self.unit, *index))
                .collect()
        });
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
        diagnostics::render(
            self.unit,
            &self.evaluation,
            self.selected_diagnostics
                .as_deref()
                .or(self.visible_diagnostics.as_deref()),
        )
    }

    fn render_patterns(&self) -> Option<String> {
        let report = self.patterns.as_ref()?;
        Some(format!(
            "\nCompiler patterns for unit {}, source root {} (input complete={}):\n{}",
            self.unit.ordinal(),
            self.source_root.display(),
            report.input_complete,
            report.render()
        ))
    }

    fn inputs_complete(&self) -> bool {
        self.unavailable.is_empty()
            && self.unavailable_complexity.is_empty()
            && self.unresolved_callees.is_empty()
            && self
                .patterns
                .as_ref()
                .is_none_or(|report| report.input_complete)
    }

    fn findings(&self) -> usize {
        self.selected_diagnostics
            .as_ref()
            .or(self.visible_diagnostics.as_ref())
            .map_or(self.evaluation.summary.violations, Vec::len)
            + self.patterns.as_ref().map_or(0, |report| report.findings())
    }
}

fn observation(function: &super::Function, root: &std::path::Path) -> Result<Observation, String> {
    let Some(raw) = function.facts.build.frontend_options.get(KEY) else {
        return Ok(Observation::Unavailable {
            reason: "Compiler advisory observations were not captured".into(),
        });
    };
    let mut observation = Observation::read(raw)?;
    if let Observation::Known { facts, metrics } = &mut observation {
        if facts.name != function.facts.name || facts.source != function.facts.source.bytes.as_str()
        {
            return Err(
                "Compiler advisory metrics or source owner disagree with captured function".into(),
            );
        }
        let source = function
            .facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
            .ok_or("Missing compiler advisory source location")?;
        let original = std::path::Path::new(source);
        let normalized = (matches!(
            metrics.as_str(),
            rstyle_core::advisory::V5_METRICS | METRICS
        ))
        .then(|| rstyle_pdg::nominal_binding::absolute_path(original))
        .transpose()?;
        let path = normalized.as_deref().unwrap_or(original);
        facts.path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if facts.params + usize::from(facts.has_self) != function.facts.signature.params.len()
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
