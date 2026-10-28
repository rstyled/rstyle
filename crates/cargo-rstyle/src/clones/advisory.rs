//! Reuse core policy over compiler-collected inputs, isolated by compilation unit.
mod diagnostics;
mod patterns;
mod selection;
use super::{analysis::Completion, identity::UnitId, report::Context};
use rstyle_core::{
    advisory::{KEY, METRICS, Observation},
    config::Config,
    facts::FnFacts,
};
use serde::Serialize;
use std::{collections::BTreeSet, rc::Rc};

#[derive(Serialize)]
pub(super) struct Report {
    schema: &'static str,
    metrics: &'static str,
    effective_config: Rc<serde_json::Value>,
    complete: bool,
    pub findings: usize,
    limitations: &'static [&'static str],
    units: Vec<Unit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pattern_review: Option<super::review::compiler_patterns::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic_review: Option<super::review::compiler_diagnostics::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic_baseline: Option<super::review::compiler_baseline::Report>,
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
    unresolved_callees: BTreeSet<String>,
    evaluation: rstyle_core::report::Report,
    #[serde(skip)]
    diagnostic_snapshots: Vec<Result<super::review::compiler_diagnostics::Snapshot, String>>,
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
        format!("{summary}{diagnostics}{patterns}{summaries}")
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

        let selected = self
            .changed_scope
            .as_ref()
            .map_or_else(String::new, |scope| {
                scope.render_for("compiler diagnostics")
            });
        format!("{reviews}{diagnostic_reviews}{baseline}{selected}")
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
    let units = ctx
        .handoff
        .unit_contexts
        .iter()
        .map(|unit| evaluate_unit(ctx, unit, &policy))
        .collect::<Result<Vec<_>, _>>()?;
    let complete = ctx.capture.failures.is_empty()
        && ctx.handoff.failures.is_empty()
        && !units.is_empty()
        && units.iter().all(Unit::inputs_complete);

    Ok(Some(Report {
        schema: "rstyle.compiler-advisory/1",
        metrics: METRICS,
        effective_config: policy,
        complete,
        findings: units.iter().map(Unit::findings).sum(),
        limitations: &[
            "Advisory compiler observations and existing core policy; no strict clone or purity authority.",
            "Each compilation unit is evaluated separately; cross-package propagation is unavailable.",
            "Missing functions and unresolved workspace callees make coverage incomplete; no zero facts are substituted.",
            "Core metrics count mutable borrows and use bounded effect propagation; they differ from strict mutation-effect metrics.",
            "Compiler patterns use approximate advisory graphs and bounded core discovery; input completeness is not exhaustive proposal coverage or refactoring safety.",
        ],
        units,
        pattern_review: None,
        diagnostic_review: None,
        diagnostic_baseline: None,
        changed_scope: None,
    }))
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

fn evaluate_unit(
    ctx: &Context<'_>,
    unit: &super::identity::UnitContext,
    policy: &Rc<serde_json::Value>,
) -> Result<Unit, String> {
    let captured = &ctx.capture.units[unit.observed_unit];
    let root = captured.target.manifest_dir.clone();
    let inputs = collect_inputs(ctx, unit.id)?;
    let Collected {
        facts,
        observations,
        unavailable,
        missing_graphs,
    } = inputs;
    let unresolved_callees = unresolved(&facts)?;
    let expected_functions = (ctx.capture.failures.is_empty() && ctx.handoff.failures.is_empty())
        .then_some(observations.len() + unavailable.len());
    let diagnostics::Completed {
        evaluation,
        facts,
        snapshots: diagnostic_snapshots,
    } = diagnostics::evaluate(ctx, unit, facts, policy)?;
    let patterns = ctx.args.compiler_patterns.then(|| {
        patterns::evaluate(
            ctx,
            unit,
            patterns::Inputs {
                facts,
                missing: missing_graphs,
                unavailable_functions: unavailable.len(),
            },
        )
    });

    Ok(Unit {
        unit: unit.id,
        package_root: root,
        source_root: ctx.capture.plan.root.clone(),
        expected_functions,
        observations,
        unavailable,
        unresolved_callees,
        evaluation,
        diagnostic_snapshots,
        visible_diagnostics: None,
        selected_diagnostics: None,
        patterns,
    })
}

#[derive(Default)]
struct Collected {
    facts: Vec<FnFacts>,
    observations: Vec<Measurement>,
    unavailable: Vec<Missing>,
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
            Observation::Known { facts, .. } => {
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
        self.observations.push(measurement(function.key, &facts));
        self.facts.push(facts);
        Ok(())
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
    if let Observation::Known { metrics, facts } = &mut observation {
        if metrics != METRICS
            || facts.name != function.facts.name
            || facts.source != function.facts.source.bytes.as_str()
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
        let path = std::path::Path::new(source);
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
