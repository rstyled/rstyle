//! Reuse core policy over compiler-collected inputs, isolated by compilation unit.
mod patterns;
use super::{analysis::Completion, identity::UnitId, report::Context};
use rstyle_core::{
    advisory::{KEY, METRICS, Observation},
    config::Config,
    facts::FnFacts,
};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Serialize)]
pub(super) struct Report {
    schema: &'static str,
    metrics: &'static str,
    effective_config: serde_json::Value,
    complete: bool,
    pub findings: usize,
    limitations: &'static [&'static str],
    units: Vec<Unit>,
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
    }
}

impl Report {
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
        format!("{summary}{patterns}")
    }
}

pub(super) fn evaluate(ctx: &Context<'_>) -> Result<Option<Report>, String> {
    let Some(config) = ctx.compiler_advisory_config else {
        return Ok(None);
    };
    let units = ctx
        .handoff
        .unit_contexts
        .iter()
        .map(|unit| evaluate_unit(ctx, unit, config))
        .collect::<Result<Vec<_>, _>>()?;
    let complete = ctx.capture.failures.is_empty()
        && ctx.handoff.failures.is_empty()
        && !units.is_empty()
        && units.iter().all(Unit::inputs_complete);

    Ok(Some(Report {
        schema: "rstyle.compiler-advisory/1",
        metrics: METRICS,
        effective_config: serde_json::to_value(config).map_err(|error| error.to_string())?,
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
    config: &Config,
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
    let (evaluation, facts) = rstyle_core::evaluate_and_retain(facts, config)?;
    let patterns = ctx
        .args
        .compiler_patterns
        .then(|| patterns::evaluate(facts, missing_graphs, unavailable.len()));

    Ok(Unit {
        unit: unit.id,
        package_root: root,
        source_root: ctx.capture.plan.root.clone(),
        expected_functions,
        observations,
        unavailable,
        unresolved_callees,
        evaluation,
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
    fn render_patterns(&self) -> Option<String> {
        let report = self.patterns.as_ref()?;
        Some(format!(
            "\nCompiler patterns for {:?}, source root {} (input complete={}):\n{}",
            self.unit,
            self.source_root.display(),
            report.input_complete,
            rstyle_core::patterns_report::text(&report.evaluation)
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
        self.evaluation.summary.violations
            + self
                .patterns
                .as_ref()
                .map_or(0, |report| report.evaluation.candidates.len())
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
