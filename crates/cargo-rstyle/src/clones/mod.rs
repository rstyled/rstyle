//! Compiler capture over documents, and the reports evaluated from them.

mod advisory;
mod advisory_only;
mod analysis;
pub mod capture;
mod diagnostic_selection;
mod documents;
pub(crate) use advisory::runtime_bounds::from_records as document_bounds;
mod identity;
pub(crate) mod patterns_route;
mod replay;
mod report;
mod review;
mod selected_context;
pub(crate) mod style_route;
use crate::Cli;
use clap::Args as ClapArgs;
use identity::FunctionKey;
use rstyle_pdg::diagnostics::Stage;
use serde::{Serialize, de::DeserializeOwned};
#[cfg(test)]
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Requested consumer population; never changes raw capture or strict matching.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum, Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompilerSourceScope {
    #[default]
    Captured,
    Authored,
}

impl CompilerSourceScope {
    fn is_default(&self) -> bool {
        *self == Self::Captured
    }
}

#[derive(ClapArgs, Serialize)]
#[command(
    group(clap::ArgGroup::new("compiler_presentation").args(["min_score", "top"]).requires("compiler_patterns").multiple(true)),
    group(clap::ArgGroup::new("compiler_mining").args(["hide_implements", "threshold", "min_coverage", "pair_coverage", "normalize", "single_call_guard", "max_holes"]).requires("compiler_patterns").multiple(true)),
    group(clap::ArgGroup::new("compiler_inputs").args(["compiler_advisory", "compiler_patterns", "compiler_complexity_top", "compiler_runtime_bounds"]).multiple(true)),
    group(clap::ArgGroup::new("advisory_inputs").args(["compiler_advisory", "compiler_patterns"]).multiple(true)),
    group(clap::ArgGroup::new("review_analysis").args(["compiler_advisory", "compiler_patterns"]).multiple(true)),
    group(clap::ArgGroup::new("changed_analysis").args(["compiler_advisory", "compiler_patterns"]).multiple(true)),
    group(clap::ArgGroup::new("dismissal").args(["dismiss_compiler_diagnostic", "dismiss_compiler_pattern"]).multiple(true)),
    group(clap::ArgGroup::new("finding_analysis").args(["compiler_advisory", "compiler_patterns"]).multiple(true)),
)]
pub struct Args {
    /// Retain only the compact member census for the root repository-review session.
    #[arg(skip)]
    #[serde(skip)]
    pattern_session: bool,
    /// Strictly reevaluate an exact saved standalone run or dependency closure without compiling.
    #[arg(long)]
    replay_run: Option<PathBuf>,
    /// Report core's numeric diagnostics over compiler observations.
    #[arg(long)]
    pub compiler_advisory: bool,
    /// Select core numeric rules by ID; omitted means all rules. Invalid suppressions always surface.
    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        requires = "compiler_advisory"
    )]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    compiler_diagnostic_rules: Vec<diagnostic_selection::Rule>,
    /// Diagnostic population; authored lists generated missing owners outside the requested scope.
    #[arg(long, value_enum, default_value_t = CompilerSourceScope::Captured, requires = "compiler_advisory")]
    #[serde(skip_serializing_if = "CompilerSourceScope::is_default")]
    compiler_diagnostic_scope: CompilerSourceScope,
    /// Analyze symbolic loop-body entry and call invocation upper bounds.
    #[arg(long, requires = "compiler_advisory")]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    compiler_runtime_bounds: bool,
    /// Maximum runtime-bound rows displayed per compilation unit.
    #[arg(long, requires = "compiler_runtime_bounds")]
    #[serde(skip_serializing_if = "Option::is_none")]
    compiler_runtime_top: Option<usize>,
    /// Rank up to N compiler functions per unit by existing COG then CC scores, including clean functions.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    compiler_complexity_top: Option<std::num::NonZeroUsize>,
    /// Rank captured inputs or authored functions, retaining generated owners outside the scope.
    #[arg(long, value_enum, default_value_t = CompilerSourceScope::Captured, requires = "compiler_complexity_top")]
    #[serde(skip_serializing_if = "CompilerSourceScope::is_default")]
    compiler_complexity_scope: CompilerSourceScope,
    /// Reuse compiler graphs and core proposals independently of numeric style diagnostics.
    #[arg(long)]
    compiler_patterns: bool,
    /// Select captured inputs or authored functions, explicitly excluding expansion signatures.
    #[arg(long, value_enum, default_value_t = CompilerSourceScope::Captured, requires = "compiler_patterns")]
    #[serde(skip_serializing_if = "CompilerSourceScope::is_default")]
    compiler_pattern_scope: CompilerSourceScope,
    /// Apply the repository candidate ledger using its existing exact and growth rules.
    #[arg(long, requires = "compiler_patterns")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pattern_ledger: Option<PathBuf>,
    /// Include repository-dismissed compiler proposals in text output.
    #[arg(long, requires = "compiler_patterns")]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    show_ignored: bool,
    #[command(flatten, next_help_heading = "Compiler pattern mining")]
    #[serde(skip_serializing_if = "crate::MineArgs::is_default")]
    mine: crate::MineArgs,
    #[command(flatten, next_help_heading = "Compiler pattern presentation")]
    #[serde(skip_serializing_if = "crate::PatternDisplayArgs::is_default")]
    pattern_display: crate::PatternDisplayArgs,
    /// Select compiler proposals touching changed files at any usage site or definition.
    #[arg(long, requires_all = ["compiler_patterns", "changed_files"])]
    select_compiler_patterns: bool,
    /// Reuse core fix groups and parameter plans across captured dependency units; advisory only.
    #[arg(long, requires = "compiler_advisory")]
    compiler_plans: bool,
    /// Inspect a saved-report function and its known potential callers, including clean functions.
    #[arg(long, requires_all = ["compiler_plans", "replay_run"])]
    #[serde(skip_serializing_if = "Option::is_none")]
    compiler_callers: Option<FunctionKey>,
    /// Reverse call reach for --compiler-callers; defaults to one hop, zero selects only the seed.
    #[arg(long, requires = "compiler_callers")]
    #[serde(skip_serializing_if = "Option::is_none")]
    compiler_caller_hops: Option<usize>,
    /// TOML policy for compiler metrics, proposal effects and exclusions; independent of strict clones.
    #[arg(long, requires = "compiler_inputs")]
    compiler_advisory_config: Option<PathBuf>,
    /// JSON workspace-relative changed-file selection for source, density and compiler diagnostic presentation and finding exits.
    #[arg(long, requires = "changed_analysis")]
    changed_files: Option<PathBuf>,
    /// Compare completed compiler diagnostics with an explicit saved inventory.
    #[arg(
        long,
        requires = "compiler_advisory",
        conflicts_with = "write_compiler_baseline"
    )]
    compiler_baseline: Option<PathBuf>,
    /// Export a complete compiler diagnostic inventory to a new file.
    #[arg(long, requires = "compiler_advisory", conflicts_with_all = ["changed_files", "compiler_baseline"])]
    write_compiler_baseline: Option<PathBuf>,
    /// Fail for new or changed compiler diagnostics, or policy/build scope drift.
    #[arg(long, requires = "compiler_baseline")]
    fail_on_new_compiler: bool,
    /// Compare compiler proposals with an explicit saved inventory.
    #[arg(
        long,
        requires = "compiler_patterns",
        conflicts_with = "write_compiler_pattern_baseline"
    )]
    compiler_pattern_baseline: Option<PathBuf>,
    /// Export a complete compiler proposal inventory to a new file.
    #[arg(long, requires = "compiler_patterns", conflicts_with_all = ["changed_files", "compiler_pattern_baseline"])]
    write_compiler_pattern_baseline: Option<PathBuf>,
    /// Fail for new or changed compiler proposals, or policy/build scope drift.
    #[arg(long, requires = "compiler_pattern_baseline")]
    fail_on_new_compiler_patterns: bool,
    /// Exit 1 when requested analyses find opportunities; incomplete analysis exits 2.
    #[arg(long, requires = "finding_analysis")]
    fail_on_findings: bool,
    /// Apply versioned reviews for requested abstraction families; lookup never writes.
    #[arg(long, requires = "review_analysis")]
    review_file: Option<PathBuf>,
    /// Inspect every retained review; omitted or unavailable assessments remain incomplete.
    #[arg(long, requires = "review_file", conflicts_with = "dismissal")]
    review_status: bool,
    /// Remove only completed negative reviews from an explicit saved run; implies status.
    #[arg(long, requires_all = ["review_file", "replay_run"], conflicts_with = "dismissal")]
    review_prune: bool,
    /// Preview review pruning without writing or creating a review file.
    #[arg(long, requires = "review_prune")]
    review_prune_dry_run: bool,
    /// Dismiss a saved-run compiler diagnostic by numeric unit and zero-based index.
    #[arg(long, num_args = 2, value_names = ["UNIT", "DIAGNOSTIC_INDEX"], requires_all = ["review_file", "review_reason", "replay_run", "compiler_advisory"])]
    dismiss_compiler_diagnostic: Vec<String>,
    /// Dismiss one saved-run compiler proposal selected by numeric unit and candidate ID.
    #[arg(long, num_args = 2, value_names = ["UNIT", "CANDIDATE_ID"], requires_all = ["review_file", "review_reason", "replay_run", "compiler_patterns"])]
    dismiss_compiler_pattern: Vec<String>,
    /// Reason retained with an explicitly dismissed finding.
    #[arg(long, requires = "dismissal")]
    review_reason: Option<String>,
    /// Retain review for future proven membership growth with unchanged original members.
    #[arg(long, requires = "dismissal")]
    review_allow_growth: bool,
    #[arg(long)]
    pub release: bool,
    #[arg(long)]
    pub locked: bool,
    #[arg(long)]
    pub offline: bool,
    #[arg(long)]
    pub features: Option<String>,
    #[arg(long)]
    pub all_features: bool,
    #[arg(long)]
    pub no_default_features: bool,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(short, long)]
    pub jobs: Option<usize>,
}

impl Args {
    fn review_status_requested(&self) -> bool {
        self.review_status || self.review_prune
    }
}

#[cfg(test)]
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

struct Configuration {
    pattern_ledger: Option<rstyle_core::ledger::Ledger>,
    compiler_advisory: Option<rstyle_core::config::Config>,
    analysis: Option<analysis::Config>,
    compiler_baseline: Option<review::compiler_baseline::Baseline>,
    compiler_pattern_baseline: Option<review::compiler_pattern_baseline::Baseline>,
}

// rstyle-allow(side_effect_density): Configuration boundary loads explicit policy, ledger and baselines.
fn configuration(cli: &Cli, args: &Args) -> Result<Configuration, String> {
    if cli.config.is_some() || cli.ignore_file.is_some() {
        return Err("This route does not consume root --config or --ignore-file".into());
    }
    Ok(Configuration {
        pattern_ledger: args
            .pattern_ledger
            .as_deref()
            .map(crate::ignore::load)
            .transpose()?,
        compiler_advisory: advisory::configuration(args)?,
        compiler_pattern_baseline: args
            .compiler_pattern_baseline
            .as_deref()
            .map(review::compiler_pattern_baseline::load)
            .transpose()?,
        compiler_baseline: args
            .compiler_baseline
            .as_deref()
            .map(review::compiler_baseline::load)
            .transpose()?,
        analysis: analysis::configuration(args)?,
    })
}

struct Function {
    key: FunctionKey,
    compilation_unit: String,
    unit_evidence: UnitEvidence,
    facts: ReportFacts,
}

#[derive(Serialize)]
struct UnitEvidence {
    report_unit: identity::UnitId,
    bundle: PathBuf,
    function: u32,
}

/// One function of its unit's documents.
struct ReportFacts {
    owner: Arc<Vec<documents::DocumentFunction>>,
    index: usize,
}

/// Borrowed navigation and compiler observations of one document function.
struct FactView<'a> {
    name: &'a str,
    fn_span: rstyle_pdg::ByteSpan,
    digest: &'a str,
    signature: &'a rstyle_core::cpg::records::DeclaredSignature,
    build: &'a rstyle_pdg::BuildFacts,
    source_file: Option<&'a str>,
    /// A macro writes the signature.
    generated: bool,
}

impl<'a> From<&'a documents::DocumentFunction> for FactView<'a> {
    fn from(function: &'a documents::DocumentFunction) -> Self {
        let record = &function.record;
        Self {
            name: &record.name,
            fn_span: rstyle_pdg::ByteSpan {
                start: record.located.start as u32,
                end: record.located.end as u32,
            },
            digest: &function.digest,
            signature: &record.signature,
            build: &function.build,
            source_file: function.source_file.as_deref(),
            generated: function.generated,
        }
    }
}

impl ReportFacts {
    fn view(&self) -> FactView<'_> {
        (&self.owner[self.index]).into()
    }

    /// The function as its document records it.
    fn document(&self) -> &documents::DocumentFunction {
        &self.owner[self.index]
    }

    /// The identity of the function's measured facts, where it has any.
    fn compiled_id(&self) -> Option<&str> {
        self.compiled(|facts| &facts.id)
    }

    /// The path the function's measured facts name, where it has any.
    fn compiled_path(&self) -> Option<&str> {
        self.compiled(|facts| &facts.path)
    }

    /// A text field of the function's measured facts.
    fn compiled(&self, known: fn(&rstyle_core::facts::FnFacts) -> &str) -> Option<&str> {
        match &self.document().record.measured {
            rstyle_core::cpg::records::Measured::Known(facts) => Some(known(facts)),
            _ => None,
        }
    }
}

#[derive(Serialize)]
struct UnitBuildContexts {
    unit_bundle: PathBuf,
    builds: rstyle_pdg::nominal_binding::BuildContexts,
}

struct Handoff {
    build_contexts: Vec<UnitBuildContexts>,
    assessed: Vec<Function>,
    failures: Vec<String>,
    keys: BTreeSet<FunctionKey>,
    unit_contexts: Vec<identity::UnitContext>,
}

struct UnitRequest<'a> {
    id: identity::UnitId,
    unit: &'a capture::Unit,
    capture: &'a capture::Capture,
    args: &'a Args,
}

impl Function {}

impl Handoff {
    fn new(assignments: Option<&[identity::Assignment<'_>]>) -> Self {
        Self {
            build_contexts: Vec::new(),
            assessed: Vec::new(),
            failures: Vec::new(),
            keys: BTreeSet::new(),
            unit_contexts: assignments
                .map(|units| {
                    units
                        .iter()
                        .map(|unit| identity::UnitContext {
                            id: unit.context.id,
                            observed_unit: unit.context.observed_unit,
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn sorted(mut self) -> Self {
        self.assessed.sort_by_key(|function| function.key);
        self
    }

    fn capture(capture: &capture::Capture, args: &Args) -> Result<Self, String> {
        Stage::new("cli.handoff", "run", None).observe(|| Ok(handoff(capture, args)))
    }
}

pub fn run(cli: &Cli, args: &Args) -> Result<(String, u8), String> {
    with_capture_verification(args.replay_run.is_some(), || {
        let config = configuration(cli, args)?;
        let capture = capture_input(cli, args)?;
        run_captured(cli, args, config, capture)
    })
}

fn with_capture_verification<T>(replay: bool, execute: impl FnOnce() -> T) -> T {
    if replay {
        rstyle_pdg::verification_policy::strict(execute)
    } else {
        rstyle_pdg::verification_policy::generated(execute)
    }
}

fn run_captured(
    cli: &Cli,
    args: &Args,
    config: Configuration,
    capture: capture::Capture,
) -> Result<(String, u8), String> {
    run_captured_with(cli, args, CapturedRun { config, capture }, |report| {
        report.publish(cli.format)
    })
}

struct CapturedRun {
    config: Configuration,
    capture: capture::Capture,
}

fn run_captured_with<T>(
    cli: &Cli,
    args: &Args,
    captured: CapturedRun,
    publish: impl FnOnce(report::Report<'_>) -> Result<T, String>,
) -> Result<(T, u8), String> {
    let CapturedRun { config, capture } = captured;
    let handoff = Handoff::capture(&capture, args)?;
    let complete = capture.failures.is_empty() && handoff.failures.is_empty();
    let context = report::Context {
        cli,
        args,
        capture: &capture,
        handoff: &handoff,
        complete,
        compiler_advisory_config: config.compiler_advisory.as_ref(),
        analysis_config: config.analysis.as_ref(),
        compiler_baseline: config.compiler_baseline.as_ref(),
        compiler_pattern_baseline: config.compiler_pattern_baseline.as_ref(),
        pattern_ledger: config.pattern_ledger.as_ref(),
    };
    let consumers = report::Consumers::new(&context)?;
    let report = report::make(context, consumers);
    let finished = complete && analyses_complete(&report);
    publish_baselines(&report, args, finished)?;
    publish_compiler_pattern_baseline(&report, args, finished)?;
    let code = if finished {
        u8::from(finding_exit_requested(&report, args))
    } else {
        2
    };

    Ok((publish(report)?, code))
}

fn publish_baselines(
    report: &report::Report<'_>,
    args: &Args,
    complete: bool,
) -> Result<(), String> {
    review::baseline_common::publish_optional(
        report
            .compiler_advisory
            .as_ref()
            .and_then(advisory::Report::baseline),
        args.write_compiler_baseline.as_deref(),
        complete,
    )
}

fn publish_compiler_pattern_baseline(
    report: &report::Report<'_>,
    args: &Args,
    complete: bool,
) -> Result<(), String> {
    review::baseline_common::publish_optional(
        report
            .compiler_advisory
            .as_ref()
            .and_then(advisory::Report::pattern_baseline),
        args.write_compiler_pattern_baseline.as_deref(),
        complete,
    )
}

fn analyses_complete(report: &report::Report<'_>) -> bool {
    report
        .compiler_advisory
        .as_ref()
        .is_none_or(analysis::Completion::permits_success)
        && report
            .review_status
            .as_ref()
            .is_none_or(analysis::Completion::permits_success)
}

fn finding_exit_requested(report: &report::Report<'_>, args: &Args) -> bool {
    let advisory = report.compiler_advisory.as_ref();
    (args.fail_on_new_compiler_patterns
        && advisory
            .and_then(advisory::Report::pattern_baseline)
            .is_some_and(review::compiler_pattern_baseline::Report::gate_fails))
        || (args.fail_on_new_compiler
            && advisory
                .and_then(advisory::Report::baseline)
                .is_some_and(review::compiler_baseline::Report::gate_fails))
        || (args.fail_on_findings && advisory.is_some_and(|report| report.finding_exit_count() > 0))
}

fn capture_input(cli: &Cli, args: &Args) -> Result<capture::Capture, String> {
    if let Some(directory) = &args.replay_run {
        if cli.manifest_path.is_some() || !cli.packages.is_empty() || cli.all_targets {
            return Err("Saved-run replay uses its original selection; new manifest, package and all-targets flags are not accepted".into());
        }
        return replay::load_capture(directory, args);
    }
    capture_fresh(cli, args)
}

fn capture_fresh(cli: &Cli, args: &Args) -> Result<capture::Capture, String> {
    let capture = prepare_capture(cli, args)?;
    Ok(compile_capture(capture, cli, args))
}

fn prepare_capture(cli: &Cli, args: &Args) -> Result<capture::Capture, String> {
    Stage::new("cli.prepare", "run", None).observe(|| capture::prepare(cli, args))
}

fn compile_capture(mut capture: capture::Capture, cli: &Cli, args: &Args) -> capture::Capture {
    if let Err(error) = capture::compile(&mut capture, cli, args) {
        capture.failures.push(error);
    }
    capture
}

fn verify_frontend_build(
    build: &rstyle_pdg::BuildFacts,
    unit: &capture::Unit,
    capture: &capture::Capture,
    selected_context: &selected_context::SelectedContext,
) -> Result<(), String> {
    let value = build
        .frontend_options
        .get("invocation")
        .ok_or("missing captured invocation")?;

    selected_context::invocation(value, &unit.invocation)?;
    if build.rustc_version.known_value() != capture.plan.compiler_identity.lines().next() {
        return Err("frontend invocation or pinned compiler identity disagrees with run".into());
    }
    selected_context.check(
        build
            .frontend_options
            .get(rstyle_pdg::nominal_boundary::CONTEXT_KEY),
    )?;
    let expected = capture
        .plan
        .compiler_identity
        .lines()
        .find_map(|line| line.strip_prefix("commit-hash: "))
        .ok_or("Missing planned compiler commit identity")?;
    if build.rustc_commit.known_value() != Some(expected) {
        return Err("Data compiler commit disagrees with frozen compiler identity".into());
    }
    Ok(())
}

fn handoff(capture: &capture::Capture, args: &Args) -> Handoff {
    let assignments = identity::assign(&capture.units);
    let mut result = Handoff::new(assignments.as_deref().ok());
    if !capture.failures.is_empty() {
        return result;
    }
    let assignments = match assignments {
        Ok(assignments) => assignments,
        Err(error) => {
            return Handoff {
                failures: vec![error],
                ..result
            };
        }
    };
    for assignment in assignments {
        let request = UnitRequest {
            id: assignment.context.id,
            unit: assignment.unit,
            capture,
            args,
        };
        if let Err(error) = advisory_only::consume(&request, &mut result) {
            result.failures.push(error);
        }
    }
    result.sorted()
}

enum CompilerBaseline {
    Diagnostics,
    Patterns,
}

impl Args {
    fn compiler_inputs_requested(&self) -> bool {
        self.compiler_effects_requested()
            || self.compiler_complexity_top.is_some()
            || self.compiler_runtime_bounds
    }

    fn compiler_effects_requested(&self) -> bool {
        self.compiler_advisory || self.compiler_patterns
    }

    fn compiler_baseline_requested(&self, family: CompilerBaseline) -> bool {
        let (import, export) = match family {
            CompilerBaseline::Diagnostics => {
                (&self.compiler_baseline, &self.write_compiler_baseline)
            }
            CompilerBaseline::Patterns => (
                &self.compiler_pattern_baseline,
                &self.write_compiler_pattern_baseline,
            ),
        };
        import.is_some() || export.is_some()
    }
}
